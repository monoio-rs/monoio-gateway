use std::{fmt::Display, future::Future, time::Duration};

use monoio_gateway_core::{
    error::GError,
    service::{Layer, Service},
};
#[derive(Clone)]
pub struct TimeoutService<T> {
    inner: T,
    timeout: Duration,
}

impl<R, T> Service<R> for TimeoutService<T>
where
    T: Service<R>,
    T::Error: Display,
    R: 'static,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>> + 'cx
    where
        Self: 'cx;

    fn call(&mut self, req: R) -> Self::Future<'_> {
        async {
            let result = monoio::time::timeout(self.timeout, self.inner.call(req)).await;
            match result {
                Ok(Ok(resp)) => Ok(Some(resp)),
                Ok(Err(err)) => Err(anyhow::anyhow!("{}", err)),
                Err(_) => Err(anyhow::anyhow!("timeout")),
            }
        }
    }
}

pub struct TimeoutLayer {
    timeout: Duration,
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = TimeoutService<S>;

    fn layer(&self, service: S) -> Self::Service {
        TimeoutService {
            inner: service,
            timeout: self.timeout,
        }
    }
}

impl TimeoutLayer {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}
