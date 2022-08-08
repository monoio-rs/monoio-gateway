use std::{
    future::Future,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
    vec,
};

use monoio_gateway_core::error::GError;

use crate::{
    config::{Config, ProxyConfig},
    dns::{http::Domain, tcp::TcpAddress, Resolvable},
    proxy::{h1::HttpProxy, tcp::TcpProxy, Proxy},
};

pub struct GatewayAgent<'cx, Addr>
where
    Addr: Resolvable<Error = GError> + 'cx,
    Addr::Item<'cx>: ToSocketAddrs + 'cx,
    Addr::ResolveFuture<'cx>: Future<Output = Result<Option<SocketAddr>, GError>>,
{
    config: Config<'cx, Addr>,
    gateways: Vec<Gateway<'cx, Addr>>,

    phantom_data: PhantomData<&'cx Addr>,
}

pub trait Gatewayable<'cx, Addr>
where
    Addr: Resolvable<Error = GError> + 'cx,
    Addr::Item<'cx>: ToSocketAddrs + 'cx,
    Addr::ResolveFuture<'cx>: Future<Output = Result<Option<SocketAddr>, GError>>,
{
    type GatewayFuture: Future<Output = Result<(), GError>>
    where
        Self: 'cx;

    fn new(config: ProxyConfig<'cx, Addr>) -> Self;

    fn serve(&'cx self) -> Self::GatewayFuture;
}

#[derive(Clone)]
pub struct Gateway<'cx, Addr> {
    config: ProxyConfig<'cx, Addr>,

    phantom_data: PhantomData<&'cx Addr>,
}

impl<'cx> Gatewayable<'cx, TcpAddress> for Gateway<'cx, TcpAddress> {
    type GatewayFuture = impl Future<Output = Result<(), GError>> where Self: 'cx;

    fn new(config: ProxyConfig<'cx, TcpAddress>) -> Self {
        Self {
            config,
            phantom_data: PhantomData,
        }
    }

    fn serve(&'cx self) -> Self::GatewayFuture {
        async move {
            let mut proxy = TcpProxy::build_with_config(&self.config);
            proxy.io_loop().await
        }
    }
}

impl<'cx> Gatewayable<'cx, Domain> for Gateway<'cx, Domain> {
    type GatewayFuture = impl Future<Output = Result<(), GError>> where Self: 'cx;

    fn new(config: ProxyConfig<'cx, Domain>) -> Self {
        Self {
            config,
            phantom_data: PhantomData,
        }
    }

    fn serve(&'cx self) -> Self::GatewayFuture {
        async move {
            let mut proxy = HttpProxy::build_with_config(&self.config);
            proxy.io_loop().await
        }
    }
}

pub trait GatewayAgentable {
    type Config;
    type Future<'cx>: Future<Output = Result<(), anyhow::Error>>
    where
        Self: 'cx;

    fn build(config: &Self::Config) -> Self;

    fn serve(&'_ mut self) -> Self::Future<'_>;
}

impl GatewayAgentable for GatewayAgent<'static, TcpAddress> {
    type Config = Config<'static, TcpAddress>;

    type Future<'g> = impl Future<Output = Result<(), anyhow::Error>>
    where
        Self: 'g;

    fn build(config: &Self::Config) -> Self {
        let gateways: Vec<Gateway<TcpAddress>> = config
            .proxies
            .iter()
            .map(|proxy_config| Gateway::new(proxy_config.clone()))
            .collect();
        GatewayAgent {
            config: config.clone(),
            gateways,
            phantom_data: PhantomData,
        }
    }

    fn serve(&'_ mut self) -> Self::Future<'_> {
        async {
            let mut handlers = vec![];
            for gw in self.gateways.iter_mut() {
                let clone = gw.clone();
                let f = monoio::spawn(async move {
                    match clone.serve().await {
                        Ok(()) => {}
                        Err(err) => eprintln!("Error: {}", err),
                    }
                });
                handlers.push(f);
            }
            for handle in handlers {
                let _ = handle.await;
            }
            Ok(())
        }
    }
}

impl GatewayAgentable for GatewayAgent<'static, Domain> {
    type Config = Config<'static, Domain>;

    type Future<'cx> = impl Future<Output = Result<(), anyhow::Error>>
    where
        Self: 'cx;

    fn build(config: &Self::Config) -> Self {
        let gateways: Vec<Gateway<Domain>> = config
            .proxies
            .iter()
            .map(|proxy_config| Gateway::new(proxy_config.clone()))
            .collect();
        GatewayAgent {
            config: config.clone(),
            gateways,
            phantom_data: PhantomData,
        }
    }

    fn serve(&'_ mut self) -> Self::Future<'_> {
        async {
            let mut handlers = vec![];
            for gw in self.gateways.iter_mut() {
                let clone = gw.clone();
                let f = monoio::spawn(async move {
                    match clone.serve().await {
                        Ok(()) => {}
                        Err(e) => eprintln!("Error: {}", e),
                    }
                });
                handlers.push(f);
            }
            for handle in handlers {
                let _ = handle.await;
            }
            Ok(())
        }
    }
}
