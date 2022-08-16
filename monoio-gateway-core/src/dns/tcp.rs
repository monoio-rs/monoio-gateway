use std::{
    fmt::Display,
    future::Future,
    net::{SocketAddr, ToSocketAddrs},
    option,
};

use serde::{Deserialize, Serialize};

use super::Resolvable;

#[derive(Copy, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
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

    type Address = SocketAddr;

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

impl ToSocketAddrs for TcpAddress {
    type Iter = option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        self.inner.to_socket_addrs()
    }
}
