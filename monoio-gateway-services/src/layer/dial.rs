use std::future::Future;

use anyhow::bail;
use log::info;
use monoio::net::TcpStream;
use monoio_gateway_core::{
    dns::Resolvable,
    error::GError,
    service::{Layer, Service},
};

use super::{accept::TcpAccept, transfer::TransferParams};
#[derive(Clone)]
pub struct DialRemote<T, A> {
    inner: T,
    target: A,
}

impl<T, A> Service<TcpAccept> for DialRemote<T, A>
where
    T: Service<TransferParams>,
    A: Resolvable,
{
    type Response = T::Response;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, local_stream: TcpAccept) -> Self::Future<'_> {
        async {
            info!("dailing remote");
            match self.target.resolve().await {
                Ok(Some(domain)) => match TcpStream::connect(domain).await {
                    Ok(remote_stream) => {
                        match self.inner.call((local_stream.0, remote_stream)).await {
                            Ok(resp) => Ok(resp),
                            Err(err) => bail!("{}", err),
                        }
                    }
                    Err(err) => {
                        bail!("error connect to remote: {}", err)
                    }
                },
                _ => {
                    bail!("error resolve remote domain: {}", self.target)
                }
            }
        }
    }
}

pub struct DialRemoteLayer<A> {
    target: A,
}

impl<A> DialRemoteLayer<A> {
    pub fn new(addr: A) -> Self {
        Self { target: addr }
    }
}

impl<S, A> Layer<S> for DialRemoteLayer<A>
where
    A: Resolvable,
{
    type Service = DialRemote<S, A>;

    fn layer(&self, service: S) -> Self::Service {
        DialRemote {
            inner: service,
            target: self.target.clone(),
        }
    }
}
