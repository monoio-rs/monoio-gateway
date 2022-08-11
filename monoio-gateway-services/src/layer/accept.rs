use std::{fmt::Display, future::Future, net::SocketAddr};

use monoio::{io::stream::Stream, net::TcpStream};
use monoio_gateway_core::{
    error::GError,
    service::{Layer, Service},
};

pub struct TcpAcceptService<T> {
    inner: T,
}

pub type TcpAccept = (TcpStream, SocketAddr);

impl<L, T> Service<L> for TcpAcceptService<T>
where
    L: Stream<Item = TcpAccept>,
    T: Service<TcpAccept>,
    T::Error: Display,
{
    type Response = T::Response;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, mut stream: L) -> Self::Future<'_> {
        async move {
            let next = stream.next().await;
            if let Some(item) = next {
                match self.inner.call(item).await {
                    Ok(resp) => Ok(resp),
                    Err(err) => Err(anyhow::anyhow!("{}", err)),
                }
            } else {
                Err(anyhow::anyhow!("error accept tcp stream"))
            }
        }
    }
}

#[derive(Default)]
pub struct TcpAcceptLayer {}

impl<S> Layer<S> for TcpAcceptLayer {
    type Service = TcpAcceptService<S>;

    fn layer(&self, service: S) -> Self::Service {
        TcpAcceptService { inner: service }
    }
}
