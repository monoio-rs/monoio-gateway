use std::future::Future;

use http::HeaderValue;
use monoio_gateway_core::{error::GError, service::Service};
use monoio_http::common::request::Request;

#[derive(Clone)]
pub struct BearerAuthService {
    token: String,
}

impl Service<Request> for BearerAuthService {
    type Response = Request;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, mut req: Request) -> Self::Future<'_> {
        async {
            let headers = req.headers_mut();
            headers.insert("Authorization", HeaderValue::from_str(&self.token).unwrap());
            Ok(req)
        }
    }
}
