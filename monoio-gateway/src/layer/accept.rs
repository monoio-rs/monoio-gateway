use std::{future::Future, net::SocketAddr};

use monoio::{io::stream::Stream, net::TcpStream};

use super::service::Service;
use anyhow::anyhow;

pub struct TcpAcceptService {}

impl<L> Service<L> for TcpAcceptService
where
    L: Stream<Item = (TcpStream, SocketAddr)>,
{
    type Response = (TcpStream, SocketAddr);

    type Error = anyhow::Error;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, mut stream: L) -> Self::Future<'_> {
        async move {
            let next = stream.next().await;
            if let Some(item) = next {
                Ok(item)
            } else {
                Err(anyhow!("error accept tcp stream"))
            }
        }
    }
}
