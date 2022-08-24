use std::future::Future;

pub mod detect;
pub mod router;
pub mod version;

pub trait Detect<I> {
    type Protocol;
    type DetectFuture<'a>: Future<Output = Result<Option<Self::Protocol>, anyhow::Error>>
    where
        Self: 'a;

    fn detect_proto(&self, io: &mut I) -> Self::DetectFuture<'_>;
}
