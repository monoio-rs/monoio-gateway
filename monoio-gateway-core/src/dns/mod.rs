use std::{
    fmt::{Debug, Display},
    future::Future,
    net::ToSocketAddrs,
};

pub mod http;
pub mod tcp;

pub trait Resolvable: Clone + Display {
    type Error: Debug;
    type Item: ToSocketAddrs;
    type ResolveFuture<'a>: Future<Output = Result<Option<Self::Item>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_>;
}
