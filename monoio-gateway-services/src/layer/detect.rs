use std::future::Future;

use monoio::io::AsyncReadRent;
use monoio_gateway_core::{http::Detect, service::Service};

#[derive(Clone)]
pub struct DetectService<D> {
    detect: D,
}

impl<I, P, D> Service<I> for DetectService<D>
where
    I: AsyncReadRent,
    D: Detect<I, Protocol = P> + Clone,
{
    type Response = Option<P>;

    type Error = anyhow::Error;

    type Future<'a> = impl Future<Output = Result<Self::Response, anyhow::Error>> where D: 'a;

    fn call(&mut self, mut io: I) -> Self::Future<'_> {
        let detect = self.detect.clone();
        async move { detect.detect_proto(&mut io).await }
    }
}
