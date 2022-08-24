use std::future::Future;

use acme_lib::Certificate;
use anyhow::bail;
use monoio_gateway_core::{
    acme::{lets_encrypt::LetsEncryptAcme, Acme},
    dns::http::Domain,
    error::GError,
    service::Service,
};

#[derive(Clone)]
pub struct LetsEncryptService {
    email: String,
}

impl Service<Domain> for LetsEncryptService {
    type Response = Option<Certificate>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: Domain) -> Self::Future<'_> {
        async {
            let acme = LetsEncryptAcme::new(req);
            match acme.acme(self.email.clone()).await {
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
