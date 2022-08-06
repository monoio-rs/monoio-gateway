use std::{future::Future, net::SocketAddr};

use monoio::{io::stream::Stream, net::TcpStream};
use monoio_gateway_core::{error::GError, service::Service};

pub struct TcpAcceptService {}

impl<L> Service<L> for TcpAcceptService
where
    L: Stream<Item = (TcpStream, SocketAddr)>,
{
    type Response = (TcpStream, SocketAddr);

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, mut stream: L) -> Self::Future<'_> {
        async move {
            let next = stream.next().await;
            if let Some(item) = next {
                Ok(item)
            } else {
                Err(anyhow::anyhow!("error accept tcp stream"))
            }
        }
    }
}
