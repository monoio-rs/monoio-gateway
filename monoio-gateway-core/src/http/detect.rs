use std::future::Future;

use monoio::io::AsyncReadRent;

use super::Detect;

#[derive(Default)]
pub struct DetectHttpVersion {}

impl<I> Detect<I> for DetectHttpVersion
where
    I: AsyncReadRent,
{
    type Protocol = DetectHttpVersion;

    type DetectFuture<'a> = impl Future<Output = Result<Option<Self::Protocol>, anyhow::Error>>;

    fn detect_proto(&self, _io: &mut I) -> Self::DetectFuture<'_> {
        // TODO
        async { Ok(None) }
    }
}
