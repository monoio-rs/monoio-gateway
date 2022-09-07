use http::HeaderValue;
use monoio_http::{
    common::{request::Request, response::Response},
    h1::payload::Payload,
};

use crate::dns::http::Domain;

pub struct Rewrite;

impl Rewrite {
    #[inline]
    pub fn rewrite_request(request: &mut Request<Payload>, remote: &Domain) {
        let authority = remote.authority();
        if authority.is_none() {
            // ignore rewrite
            return;
        }
        let new_header = HeaderValue::from_str(authority.unwrap().as_str())
            .unwrap_or(HeaderValue::from_static(""));
        log::debug!(
            "Request: {:?} -> {:?}",
            request.headers().get(http::header::HOST),
            new_header
        );
        request.headers_mut().insert(http::header::HOST, new_header);
    }

    #[inline]
    pub fn rewrite_response(response: &mut Response<Payload>, local: &Domain) {
        let authority = local.authority();
        if authority.is_none() || response.headers().get(http::header::HOST).is_none() {
            // ignore rewrite
            return;
        }
        let new_header = HeaderValue::from_str(authority.unwrap().as_str())
            .unwrap_or(HeaderValue::from_static(""));
        log::debug!(
            "Response: {:?} <- {:?}",
            new_header,
            response.headers().get(http::header::HOST)
        );
        response
            .headers_mut()
            .insert(http::header::HOST, new_header);
    }
}
