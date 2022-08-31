use std::{fmt::Display, future::Future, net::SocketAddr, rc::Rc};

use anyhow::bail;
use log::info;
use monoio::{
    net::{TcpListener, TcpStream},
};
use monoio_gateway_core::{
    error::GError,
    service::{Layer, Service},
};

#[derive(Clone)]
pub struct TcpAcceptService<T> {
    inner: T,
}

pub type Accept<S> = (S, SocketAddr);

impl<T> Service<Rc<TcpListener>> for TcpAcceptService<T>
where
    T: Service<Accept<TcpStream>>,
    T::Error: Display,
{
    type Response = T::Response;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, listener: Rc<TcpListener>) -> Self::Future<'_> {
        async move {
            match listener.accept().await {
                Ok(accept) => {
                    info!("accept a connection");
                    match self.inner.call(accept).await {
                        Err(err) => {
                            bail!("Error: {}", err);
                        }
                        Ok(resp) => return Ok(resp),
                    }
                }
                Err(err) => bail!("{}", err),
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
