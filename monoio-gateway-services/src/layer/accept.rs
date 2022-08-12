use std::{fmt::Display, future::Future, net::SocketAddr};

use anyhow::bail;
use log::info;
use monoio::net::{TcpListener, TcpStream};
use monoio_gateway_core::{
    error::GError,
    service::{Layer, Service},
};

#[derive(Clone)]
pub struct TcpAcceptService<T> {
    inner: T,
}

pub type TcpAccept = (TcpStream, SocketAddr);

impl<T> Service<TcpListener> for TcpAcceptService<T>
where
    T: Service<TcpAccept> + 'static,
    T::Error: Display,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, listener: TcpListener) -> Self::Future<'_> {
        async move {
            loop {
                let mut inner_clone = self.inner.to_owned();
                match listener.accept().await {
                    Ok(accept) => {
                        info!("accept a connection");
                        monoio::spawn(async move {
                            // let inner_svc = inner_clone;
                            match inner_clone.call(accept).await {
                                _ => {}
                            }
                        });
                    }
                    Err(err) => bail!("{}", err),
                }
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
