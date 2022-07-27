

use crate::{discover::Discover};

use super::svc_list::SvcList;

// /// Load Balancer
pub struct Balance<D, S>
where
    D: Discover,
    S: IntoIterator,
{
    discover: D,
    services: SvcList<S>,
}
