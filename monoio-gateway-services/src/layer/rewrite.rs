use std::future::Future;

use monoio::io::AsyncWriteRent;
use monoio_gateway_core::{
    dns::http::Domain,
    http::Rewrite,
    service::{Layer, Service},
};

use super::endpoint::EndpointRequestParams;

/// Rewrite header service
/// default: rewrite host field
///
/// useful for proxy pass a domain with another
#[derive(Clone)]
pub struct RewriteService<T> {
    inner: T,
}

impl<S, T> Service<EndpointRequestParams<Domain, Domain, S>> for RewriteService<T>
where
    S: AsyncWriteRent,
    T: Service<EndpointRequestParams<Domain, Domain, S>>,
{
    type Response = T::Response;

    type Error = T::Error;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, mut args: EndpointRequestParams<Domain, Domain, S>) -> Self::Future<'_> {
        if args.local_req.is_none() {
            return self.inner.call(args);
        } else {
            let request = args.local_req.as_mut().unwrap();
            Rewrite::rewrite_request(request, &args.endpoint);
            return self.inner.call(args);
        }
    }
}

#[derive(Default)]
pub struct RewriteLayer;

impl<S> Layer<S> for RewriteLayer {
    type Service = RewriteService<S>;

    fn layer(&self, service: S) -> Self::Service {
        RewriteService { inner: service }
    }
}
