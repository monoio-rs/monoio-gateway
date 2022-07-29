use std::{future::Future, net::SocketAddr};

use super::Resolvable;

#[derive(Copy, Clone)]
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

    type Item<'a> = SocketAddr;

    type ResolveFuture<'a> = impl Future<Output = Result<Option<Self::Item<'a>>, Self::Error>>;

    fn resolve(&self) -> Self::ResolveFuture<'_> {
        async { Ok(Some(self.inner.clone())) }
    }
}

// impl Resolvable for SocketAddr {}
