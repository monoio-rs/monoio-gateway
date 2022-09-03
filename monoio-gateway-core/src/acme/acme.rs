use std::{future::Future, path::Path};

use acme_lib::{create_p384_key, persist::FilePersist, Certificate, Directory, DirectoryUrl};
use anyhow::bail;
use log::{debug, info};

use crate::{acme::Acmed, error::GError, ACME_DIR, CERTIFICATE_MAP};

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
        let path_str = Path::new(&ACME_DIR.to_string())
            .join(Path::new(&self.domain))
            .join(Path::new(&format!(".well-known/acme-challenge/{}", token)));
        debug!("writing proof to {:?}", path_str);
        // create if not exist
        let path = Path::new(&path_str);
        let parent = path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent.to_owned())?;
        }
        let out = monoio::fs::File::create(path).await?;
        let _ = out.write_all_at(proof.into_bytes(), 0).await;
        out.close().await?;
        Ok(())
    }
}

impl Acme for GenericAcme {
    type Response = Certificate;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Option<Self::Response>, Self::Error>>
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
                    debug!("created new order");
                    // try [curr_times] times
                    let mut curr_times = 0;
                    let ord_csr = loop {
                        if curr_times > self.validate_retry_times {
                            bail!("acme failed after {} requests", self.validate_retry_times);
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
                    info!("validate success, downloading certificate");
                    let pkey_pri = create_p384_key();
                    let ord_cert = ord_csr.finalize_pkey(pkey_pri, self.finalize_delay)?;
                    let cert = ord_cert.download_and_save_cert()?;
                    return Ok(Some(cert));
                }
                Err(err) => {
                    debug!("get acme directory failed");
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
    let acme = GenericAcme::new_lets_encrypt_staging(server_name.to_string(), mail.to_string());
    match acme.acme(()).await {
        Ok(Some(cert)) => {
            // lint
            let cert: Certificate = cert;
            CERTIFICATE_MAP
                .write()
                .unwrap()
                .insert(server_name.to_string(), cert);
            println!("get cert, location: {:?}", location);
        }
        Err(err) => {
            bail!("{}", err)
        }
        _ => {
            // retry
        }
    }
    Ok(())
}
