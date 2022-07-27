use std::iter::Enumerate;

/// Decorates Service List
pub struct SvcList<S>
where
    S: IntoIterator,
{
    inner: Enumerate<S::IntoIter>,
}

type ListSvcList<S> = SvcList<Vec<S>>;
