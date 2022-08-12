use std::{
    fmt::{Debug, Display},
    future::Future,
    net::SocketAddr,
};

pub mod http;
pub mod tcp;

pub trait Resolvable: Clone + Display + ToSocketAddr {
    type Error: Debug;
    type ResolveFuture<'a>: Future<Output = Result<Option<SocketAddr>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_>;
}

pub trait ToSocketAddr {
    fn get_addr(&self) -> SocketAddr;
}
