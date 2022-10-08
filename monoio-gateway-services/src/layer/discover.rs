use std::{future::Future, time::Duration};

use monoio_gateway_core::{
    discover::{change::DiscoverChange, Discover},
    service::Service,
};

#[derive(Clone)]
pub struct DiscoverService<D> {
    discover: D,
    delay: Duration,
}

impl<D> Service<()> for DiscoverService<D>
where
    D: Discover + Clone,
{
    type Response = DiscoverChange<D::Key, D::Service>;

    type Error = D::Error;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, _req: ()) -> Self::Future<'_> {
        async {
            let _ = monoio::time::sleep(self.delay).await;
            let res = self.discover.discover().await;
            match res {
                Ok(Some(change)) => {
                    return Ok(change);
                }
                Ok(None) => {
                    log::info!("no endpoint discovered");
                    return Ok(DiscoverChange::None);
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }
}
