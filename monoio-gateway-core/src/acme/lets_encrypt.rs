use std::{env::join_paths, future::Future, path::Path};

use acme_lib::{create_p384_key, persist::FilePersist, Certificate, Directory, DirectoryUrl};
use anyhow::bail;
use log::{debug, info};

use crate::{acme::Acmed, dns::http::Domain, error::GError, ACME_DIR};

use super::Acme;

pub struct LetsEncryptAcme {
    domain: Domain,
    validate_delay: u64,
    finalize_delay: u64,
    validate_retry_times: u8,
}

impl LetsEncryptAcme {
    pub fn new(domain: Domain) -> Self {
        Self {
            domain,
            validate_delay: 5000,
            finalize_delay: 10000,
            validate_retry_times: 5,
        }
    }

    async fn write_proof(&self, token: &str, proof: String) -> Result<(), GError> {
        let path_str = join_paths([
            Path::new(&ACME_DIR.to_string()),
            Path::new(&self.domain.host()),
            Path::new(&format!(".well-known/acme-challenge/{}", token)),
        ])?;
        debug!("writing proof to {:?}", path_str);
        // create if not exist
        let path = Path::new(&path_str);
        let parent = path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent.to_owned())?;
        }
        let out = monoio::fs::File::open(path).await?;
        let _ = out.write_all_at(proof.into_bytes(), 0).await;
        out.close().await?;
        Ok(())
    }
}

impl Acme for LetsEncryptAcme {
    type Response = Certificate;

    /// Email
    type Email = String;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Option<Self::Response>, Self::Error>>
        where Self: 'cx;

    fn acme(&self, acme_request: Self::Email) -> Self::Future<'_> {
        async move {
            info!("request tls certificate from LetsEncrypt...");
            let url = DirectoryUrl::LetsEncrypt;
            let persist = FilePersist::new(self.domain.get_acme_path()?);
            match Directory::from_url(persist, url) {
                Ok(directory) => {
                    let acc = directory.account(&acme_request)?;
                    let mut order = acc.new_order(&self.domain.host(), &[])?;
                    let mut curr_times = 0;
                    let ord_csr = loop {
                        if curr_times > self.validate_retry_times {
                            bail!("acme failed after {} requests", self.validate_retry_times);
                        }
                        info!("try {} times for {}", curr_times, self.domain.host());
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
                Err(err) => bail!("{}", err),
            }
        }
    }
}
