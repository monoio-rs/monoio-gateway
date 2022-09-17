use std::future::Future;

use crate::error::GError;

use super::{change::DiscoverChange, Discover};

pub struct DummyDiscover<S> {
    data: S,
}

impl<S> Discover for DummyDiscover<S>
where
    S: Clone,
{
    type Key = ();

    type Service = S;

    type Error = GError;

    type DiscoverFuture<'a> = impl Future<
    Output = Result<Option<DiscoverChange<Self::Key, Self::Service>>, Self::Error>,
>
    where
        Self: 'a;

    fn discover(&self) -> Self::DiscoverFuture<'_> {
        async { Ok(Some(DiscoverChange::Add((), self.data.clone()))) }
    }
}
