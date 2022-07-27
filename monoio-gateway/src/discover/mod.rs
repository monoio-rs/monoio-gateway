use std::future::Future;

pub mod change;
pub mod discover;

pub trait Discover {
    type Service;

    type Error;

    type DiscoverFuture<'a>: Future<Output = Result<Option<Self::Service>, Self::Error>>
    where
        Self: 'a;

    fn discover(&self) -> Self::DiscoverFuture<'_>;
}
