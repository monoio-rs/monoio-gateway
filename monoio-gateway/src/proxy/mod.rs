use std::future::Future;

pub mod h1;
pub mod h2;
pub mod tcp;

pub trait Proxy {
    type Error;
    type OutputFuture<'a>: Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    fn io_loop(&mut self) -> Self::OutputFuture<'_>;
}
