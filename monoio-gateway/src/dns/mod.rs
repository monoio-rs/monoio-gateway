pub mod tcp;

pub trait Resolvable: Sized {
    type Output;
    fn resolve(&self) -> Self::Output;
}
