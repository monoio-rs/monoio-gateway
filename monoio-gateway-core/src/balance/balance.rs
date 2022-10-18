use crate::{discover::Discover, service::SvcList};

// /// Load Balancer
pub struct Balance<D, S>
where
    D: Discover,
    S: IntoIterator,
{
    pub discover: D,
    pub services: SvcList<S>,
}
