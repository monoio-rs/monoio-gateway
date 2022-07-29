use std::{fmt::Debug, future::Future, net::ToSocketAddrs};

pub mod h1;
pub mod tcp;

pub trait Resolvable: Clone {
    type Error: Debug;
    type Item<'a>: ToSocketAddrs
    where
        Self: 'a;
    type ResolveFuture<'a>: Future<Output = Result<Option<Self::Item<'a>>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_>;
}
