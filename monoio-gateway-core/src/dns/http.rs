use std::{
    fmt::{Display, Write},
    future::Future,
};

use http::{uri::Authority, Uri};
use serde::{Deserialize, Serialize};

use super::Resolvable;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct Domain {
    #[serde(with = "http_serde::uri")]
    uri: Uri,
}

impl Domain {
    pub fn new(scheme: &str, authority: &str, path: &str) -> Self {
        Self {
            uri: Uri::builder()
                .scheme(scheme)
                .authority(authority)
                .path_and_query(path)
                .build()
                .unwrap(),
        }
    }

    pub fn with_uri(uri: Uri) -> Self {
        Self { uri }
    }

    pub fn version(&self) -> crate::http::version::Type {
        let v = self.uri.scheme_str().or_else(|| Some("http")).unwrap();
        return if v == "https" {
            crate::http::version::Type::HTTPS
        } else {
            crate::http::version::Type::HTTP
        };
    }

    pub fn port(&self) -> u16 {
        match self.version() {
            crate::http::version::Type::HTTP => self.uri.port_u16().or_else(|| Some(80)).unwrap(),
            crate::http::version::Type::HTTPS => self.uri.port_u16().or_else(|| Some(443)).unwrap(),
        }
    }

    pub fn authority(&self) -> Option<&Authority> {
        self.uri.authority()
    }

    #[inline]
    pub fn host(&self) -> &str {
        self.uri.authority().unwrap().host()
    }

    pub fn listen_addr(&self, wide: bool) -> String {
        if wide {
            format!("0.0.0.0:{}", self.port())
        } else {
            format!("127.0.0.1:{}", self.port())
        }
    }
}

impl Resolvable for Domain {
    type Error = anyhow::Error;

    type Address = String;

    type ResolveFuture<'a> = impl Future<Output = Result<Option<Self::Address>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_> {
        async { Ok(Some(format!("{}:{}", self.host(), self.port()))) }
    }
}

impl Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uri)
    }
}
