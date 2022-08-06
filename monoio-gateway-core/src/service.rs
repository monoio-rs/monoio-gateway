use std::future::Future;

pub trait Service<Request> {
    /// Responses given by the service.
    type Response;
    /// Errors produced by the service.
    type Error;

    /// The future response value.
    type Future<'cx>: Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    /// Process the request and return the response asynchronously.
    fn call(&mut self, req: Request) -> Self::Future<'_>;
}

pub trait Layer<S> {
    type Service;

    fn layer(&self, inner: S) -> Self::Service;
}
