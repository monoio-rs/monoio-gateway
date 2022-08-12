use std::{fmt::Display, future::Future, iter::Enumerate};

use crate::util::{identity::Identity, stack::Stack};

pub trait Service<Request>: Clone {
    /// Responses given by the service.
    type Response;
    /// Errors produced by the service.
    type Error: Display;

    /// The future response value.
    type Future<'cx>: Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    /// Process the request and return the response asynchronously.
    fn call(&mut self, req: Request) -> Self::Future<'_>;
}

pub trait Layer<S> {
    type Service;

    fn layer(&self, service: S) -> Self::Service;
}

pub struct SvcList<S>
where
    S: IntoIterator,
{
    inner: Enumerate<S::IntoIter>,
}

type ListSvcList<S> = SvcList<Vec<S>>;

pub struct ServiceBuilder<L> {
    layer: L,
}

impl ServiceBuilder<Identity> {
    pub fn new() -> Self {
        Self {
            layer: Identity::new(),
        }
    }
}

impl Default for ServiceBuilder<Identity> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L> ServiceBuilder<L> {
    pub fn layer<T>(self, s: T) -> ServiceBuilder<Stack<T, L>> {
        ServiceBuilder {
            layer: Stack::new(s, self.layer),
        }
    }

    pub fn service<S>(&self, s: S) -> L::Service
    where
        L: Layer<S>,
    {
        self.layer.layer(s)
    }
}
