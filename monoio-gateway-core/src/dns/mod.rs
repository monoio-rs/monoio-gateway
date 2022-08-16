use std::{
    fmt::{Debug, Display},
    future::Future,
    hash::Hash,
    net::{ToSocketAddrs},
};

pub mod http;
pub mod tcp;

pub trait Resolvable: Clone + Display + PartialEq + Eq + Hash {
    type Error: Debug;
    type Address: ToSocketAddrs;
    type ResolveFuture<'a>: Future<Output = Result<Option<Self::Address>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_>;
}
