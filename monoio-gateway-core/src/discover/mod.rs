use std::{fmt::Display, future::Future};

use self::change::DiscoverChange;

pub mod change;
pub mod discover;

/// Service Discover Trait
pub trait Discover {
    type Key: Eq;

    type Service;

    type Error: Display;

    type DiscoverFuture<'a>: Future<
        Output = Result<Option<DiscoverChange<Self::Key, Self::Service>>, Self::Error>,
    >
    where
        Self: 'a;

    fn discover(&self) -> Self::DiscoverFuture<'_>;
}
