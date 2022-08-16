use std::{fmt::Display, future::Future, net::SocketAddr};

use serde::{Deserialize, Serialize};

use super::{Resolvable, ToSocketAddr};

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct TcpAddress {
    inner: SocketAddr,
}

impl TcpAddress {
    pub fn new(s: SocketAddr) -> Self {
        Self { inner: s }
    }
}

impl Resolvable for TcpAddress {
    type Error = anyhow::Error;

    type ResolveFuture<'a> = impl Future<Output = Result<Option<SocketAddr>, Self::Error>>;

    fn resolve(&self) -> Self::ResolveFuture<'_> {
        async { Ok(Some(self.inner.clone())) }
    }
}

// impl Resolvable for SocketAddr {}
impl Display for TcpAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.inner)
    }
}

impl ToSocketAddr for TcpAddress {
    fn get_addr(&self) -> SocketAddr {
        self.inner.clone()
    }
}
