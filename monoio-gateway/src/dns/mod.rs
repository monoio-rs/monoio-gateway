use std::{future::Future, net::ToSocketAddrs};

pub mod tcp;

pub trait Resolvable {
    type Error;
    type Item<'a>: ToSocketAddrs
    where
        Self: 'a;
    type ResolveFuture<'a>: Future<Output = Result<Self::Item<'a>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_>;
}
