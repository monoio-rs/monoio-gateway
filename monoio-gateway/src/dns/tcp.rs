use std::net::SocketAddr;

use super::Resolvable;


pub struct TcpAddress {
    inner: SocketAddr
}

impl Resolvable for TcpAddress
{
    type Output = SocketAddr;

    fn resolve(&self) -> Self::Output {
       self.inner.resolve()
    }
}

impl Resolvable for SocketAddr {
    type Output = SocketAddr;

    fn resolve(&self) -> Self::Output {
        self.clone()
    }
}