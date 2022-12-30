use std::future::Future;

use monoio::io::{AsyncReadRent, AsyncWriteRent, Split};
use monoio_gateway_core::{error::GError, service::Service};

use super::accept::Accept;

#[derive(Clone)]
pub struct ProxyService;

impl<S> Service<Accept<S>> for ProxyService
where
    S: Split + AsyncReadRent + AsyncWriteRent + 'static,
{
    type Response = ();

    type Error = GError;

    type Future<'a> = impl Future<Output = Result<Self::Response, Self::Error>> + 'a
    where
        Self: 'a;

    fn call(&mut self, local_stream: Accept<S>) -> Self::Future<'_> {
        async move { Ok(()) }
    }
}
