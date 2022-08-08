use monoio_gateway_core::service::SvcList;

use crate::discover::Discover;

// /// Load Balancer
pub struct Balance<D, S>
where
    D: Discover,
    S: IntoIterator,
{
    discover: D,
    services: SvcList<S>,
}
