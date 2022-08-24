use std::future::Future;

use self::change::DiscoverChange;

pub mod change;

/// Service Discover Trait
pub trait Discover {
    type Key: Eq;

    type Service;

    type Error;

    type DiscoverFuture<'a>: Future<
        Output = Result<Option<DiscoverChange<Self::Key, Self::Service>>, Self::Error>,
    >
    where
        Self: 'a;

    fn discover(&self) -> Self::DiscoverFuture<'_>;
}
