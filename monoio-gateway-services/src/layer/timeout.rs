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
            monoio::select! {
                _ = monoio::time::timeout(self.timeout, async {}) => {
                    return Err(anyhow::anyhow!("timeout"))
                }

                ret = self.inner.call(req) => {
                    return match ret {
                        Ok(resp) => Ok(Some(resp)),
                        Err(err) => Err(anyhow::anyhow!("{}", err)),
                    }
                }
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
