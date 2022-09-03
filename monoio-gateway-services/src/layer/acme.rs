use std::future::Future;

use acme_lib::Certificate;
use anyhow::bail;
use monoio_gateway_core::{
    acme::{Acme, GenericAcme},
    error::GError,
    service::Service,
};

pub type ServerName = String;
pub type Email = String;
pub type AcmeParams = (ServerName, Email);

#[derive(Clone)]
pub struct LetsEncryptService;

impl Service<AcmeParams> for LetsEncryptService {
    type Response = Option<Certificate>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: AcmeParams) -> Self::Future<'_> {
        async move {
            let acme = GenericAcme::new_lets_encrypt(req.0, req.1);
            match acme.acme(()).await {
                Ok(Some(cert)) => {
                    let cert: Certificate = cert;
                    Ok(Some(cert))
                }
                Ok(None) => Ok(None),
                Err(err) => bail!("{}", err),
            }
        }
    }
}
