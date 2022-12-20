use std::{
    future::Future,
    io::{Cursor, Write},
    path::Path,
    sync::Arc,
};

use acme_lib::{create_rsa_key, persist::FilePersist, Certificate, Directory, DirectoryUrl};
use anyhow::bail;
use log::{debug, info};
use rustls::sign::CertifiedKey;

use crate::{
    acme::Acmed,
    error::GError,
    http::ssl::{read_pem_chain, read_private_key},
    CERTIFICATE_MAP,
};

use super::Acme;

const LETSENCRYPT: &str = "https://acme-v02.api.letsencrypt.org/directory";
const LETSENCRYPT_STAGING: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";

pub struct GenericAcme {
    domain: String,
    mail: String,
    validate_delay: u64,
    finalize_delay: u64,
    validate_retry_times: u8,

    request_url: String,
}

impl GenericAcme {
    pub fn new(domain: String, request_url: String, mail: String) -> Self {
        Self {
            domain,
            mail,
            validate_delay: 5000,
            finalize_delay: 10000,
            validate_retry_times: 5,

            request_url: request_url,
        }
    }

    pub fn new_lets_encrypt(domain: String, mail: String) -> Self {
        GenericAcme::new(domain, get_lets_encrypt_url(false).to_owned(), mail)
    }

    pub fn new_lets_encrypt_staging(domain: String, mail: String) -> Self {
        GenericAcme::new(domain, get_lets_encrypt_url(true).to_owned(), mail)
    }

    async fn write_proof(&self, token: &str, proof: String) -> Result<(), GError> {
        let path_str = Path::new(&self.domain.get_acme_path()?)
            .join(Path::new(&format!(".well-known/acme-challenge/{}", token)));
        info!("writing proof to {:?}", path_str);
        // create if not exist
        let path = Path::new(&path_str);
        let parent = path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent.to_owned())?;
        }
        info!("creating challenge file {:?}", path_str);
        let mut out = std::fs::File::create(path)?;
        info!("writing challenge file {:?}", path_str);
        let _ = out.write_all(proof.as_bytes());
        info!("proof wrote to {:?}", path_str);
        Ok(())
    }
}

impl Acme for GenericAcme {
    type Response = Certificate;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Option<Self::Response>, Self::Error>> + 'cx
        where Self: 'cx;

    fn acme(&self, _: ()) -> Self::Future<'_> {
        async move {
            let url = DirectoryUrl::Other(&self.request_url);
            let persist = FilePersist::new(self.domain.get_acme_path()?);
            match Directory::from_url(persist, url) {
                Ok(directory) => {
                    // create new order
                    let acc = directory.account(&self.mail)?;
                    let mut order = acc.new_order(&self.domain, &[])?;
                    debug!("acme: created new order");
                    // try [curr_times] times
                    let mut curr_times = 0;
                    let ord_csr = loop {
                        if curr_times > self.validate_retry_times {
                            bail!("acme: failed after {} requests", self.validate_retry_times);
                        }
                        info!("try {} times for {}", curr_times, self.domain);
                        if let Some(ord_csr) = order.confirm_validations() {
                            break ord_csr;
                        }
                        // only one element per domain
                        let auths = order.authorizations()?;
                        let challenge = auths[0].http_challenge();
                        let (token, proof) = (challenge.http_token(), challenge.http_proof());
                        self.write_proof(token, proof).await?;
                        challenge.validate(self.validate_delay)?;
                        order.refresh()?;
                        curr_times += 1;
                    };
                    // validate success
                    info!("🚀 acme validate success, downloading certificate");
                    let pkey_pri = create_rsa_key(3072);
                    let ord_cert = ord_csr.finalize_pkey(pkey_pri, self.finalize_delay)?;
                    let cert = ord_cert.download_and_save_cert()?;
                    return Ok(Some(cert));
                }
                Err(err) => {
                    log::error!("get acme directory failed");
                    bail!("{}", err)
                }
            }
        }
    }
}

fn get_lets_encrypt_url(staging: bool) -> &'static str {
    if staging {
        LETSENCRYPT_STAGING
    } else {
        LETSENCRYPT
    }
}

/// This function is used to fetch Let's Encrypt Certificate from staging server
pub async fn start_acme(server_name: String, mail: String) -> Result<(), GError> {
    let location = server_name.get_acme_path()?;
    let acme = GenericAcme::new_lets_encrypt(server_name.to_string(), mail.to_string());
    match acme.acme(()).await {
        Ok(Some(cert)) => {
            // lint
            let cert: Certificate = cert;
            info!("private key: {}", cert.private_key());
            update_certificate(
                server_name.to_owned(),
                Cursor::new(cert.certificate().as_bytes()),
                Cursor::new(cert.private_key().as_bytes()),
            );
            info!("get cert, location: {:?}", location);
            // sync to disk
            save_cert_to_path(server_name, cert)?;
        }
        Err(err) => {
            bail!("{}", err)
        }
        _ => {
            // TODO: retry
        }
    }
    Ok(())
}

/// update certificate to global certificate map
pub fn update_certificate<R>(server_name: String, chain: R, priv_key: R)
where
    R: std::io::Read,
{
    log::info!("updating ssl certificate for {}", server_name);
    let key = read_private_key(priv_key);
    let chain = read_pem_chain(chain);
    if let Err(e) = key {
        log::error!("private key of {} validate failed: {}", server_name, e);
        return;
    }
    let key = rustls::sign::any_supported_type(&rustls::PrivateKey(key.unwrap()));
    let mut certs = vec![];
    if key.is_ok() && chain.is_ok() {
        let chain = chain.unwrap();
        for cert in chain.into_iter() {
            let cert = rustls::Certificate(cert);
            certs.push(cert);
        }
        let certified_key = CertifiedKey::new(certs, key.unwrap());
        CERTIFICATE_MAP
            .write()
            .unwrap()
            .insert(server_name, Arc::new(certified_key));
    } else {
        log::warn!(
            "update ssl for {} failed. chain: {}, key: {}",
            server_name,
            chain.is_ok(),
            key.is_ok()
        );
    }
}

/// save cert to disk
fn save_cert_to_path(server_name: String, cert: Certificate) -> Result<(), GError> {
    let path = server_name.get_acme_path()?;
    let pem_file_path = Path::new(&path).join("pem");
    let priv_file_path = Path::new(&path).join("priv");
    let mut pem = std::fs::File::create(pem_file_path).unwrap();
    let mut private = std::fs::File::create(priv_file_path).unwrap();
    pem.write_all(cert.certificate().as_ref())?;
    private.write_all(cert.private_key().as_ref())?;
    info!(
        "🚀 acme cert for {} is last for {} days.",
        server_name,
        cert.valid_days_left()
    );
    Ok(())
}
