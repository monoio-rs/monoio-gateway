pub mod accept;
pub mod detect;

/// monoio service layer

pub trait NewService<I> {
    type Service;

    fn new_svc(&self, inner: I) -> Self::Service;
}
