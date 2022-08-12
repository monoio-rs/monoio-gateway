use std::{future::Future, net::ToSocketAddrs};

use anyhow::bail;
use monoio::net::TcpListener;
use monoio_gateway_core::{
    config::ProxyConfig,
    dns::Resolvable,
    error::GError,
    service::{Layer, Service},
};

pub struct TcpListenService<A, T> {
    inner: T,
    proxy_config: ProxyConfig<A>,
}

impl<A, T> Service<()> for TcpListenService<A, T>
where
    A: Resolvable,
    A::Item: ToSocketAddrs,
    T: Service<TcpListener>,
{
    type Response = T::Response;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, _: ()) -> Self::Future<'_> {
        async {
            match self.proxy_config.inbound.server.addr.resolve().await {
                Ok(domain) => {
                    if let Some(socket_addr) = domain {
                        match socket_addr.to_socket_addrs() {
                            Ok(mut addr) => {
                                let listener = TcpListener::bind_with_config(
                                    addr.next().unwrap(),
                                    &self.proxy_config.listener,
                                )
                                .expect("err bind address");
                                // call listener
                                match self.inner.call(listener).await {
                                    Ok(resp) => Ok(resp),
                                    Err(err) => bail!("{}", err),
                                }
                            }
                            Err(err) => anyhow::bail!("{}", err),
                        }
                    } else {
                        anyhow::bail!("address is none")
                    }
                }
                Err(_err) => anyhow::bail!("err resolve address"),
            }
        }
    }
}

pub struct TcpListenLayer<A> {
    proxy_config: ProxyConfig<A>,
}

impl<A> TcpListenLayer<A> {
    pub fn new(proxy_config: ProxyConfig<A>) -> Self {
        TcpListenLayer { proxy_config }
    }
}

impl<A, S> Layer<S> for TcpListenLayer<A>
where
    A: Clone,
{
    type Service = TcpListenService<A, S>;

    fn layer(&self, service: S) -> Self::Service {
        TcpListenService {
            inner: service,
            proxy_config: self.proxy_config.clone(),
        }
    }
}
