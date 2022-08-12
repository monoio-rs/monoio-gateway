use std::{
    fmt::{Display, Write},
    future::Future,
};

use http::Uri;

use super::Resolvable;

#[derive(Clone)]
pub struct Domain {
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

    pub fn host(&self) -> String {
        self.uri.authority().unwrap().host().to_owned()
    }
}

impl Resolvable for Domain {
    type Error = anyhow::Error;

    type Item = String;

    type ResolveFuture<'a> = impl Future<Output = Result<Option<Self::Item>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_> {
        async {
            let host = self.uri.host();
            if let Some(host) = host {
                Ok(Some(host.to_string()))
            } else {
                Ok(None)
            }
        }
    }
}

impl Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.uri)
    }
}
