pub mod detect;
pub mod service;
pub mod svc;

/// monoio service layer

pub trait NewService<I> {
    type Service;

    fn new_svc(&self, inner: I) -> Self::Service;
}
