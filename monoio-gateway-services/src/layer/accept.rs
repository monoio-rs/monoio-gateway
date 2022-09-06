use std::{future::Future, net::SocketAddr, rc::Rc};

use anyhow::bail;
use log::info;
use monoio::net::{TcpListener, TcpStream};
use monoio_gateway_core::{
    error::GError,
    service::{Layer, Service},
};

#[derive(Default, Clone)]
pub struct TcpAcceptService;

pub type Accept<S> = (S, SocketAddr);

impl Service<Rc<TcpListener>> for TcpAcceptService {
    type Response = Accept<TcpStream>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, listener: Rc<TcpListener>) -> Self::Future<'_> {
        async move {
            log::debug!("📈 new accept avaliable, waiting");
            match listener.accept().await {
                Ok(accept) => {
                    info!("accept a connection");
                    return Ok(accept);
                }
                Err(err) => bail!("{}", err),
            }
        }
    }
}

#[derive(Default)]
pub struct TcpAcceptLayer {}

impl<S> Layer<S> for TcpAcceptLayer {
    type Service = TcpAcceptService;

    fn layer(&self, _service: S) -> Self::Service {
        TcpAcceptService {}
    }
}
