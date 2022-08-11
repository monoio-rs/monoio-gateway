use std::{future::Future, vec};

use monoio_gateway_core::{
    config::{Config, InBoundConfig, OutBoundConfig, ProxyConfig},
    dns::{http::Domain, tcp::TcpAddress},
    error::GError,
};

use crate::proxy::{h1::HttpProxy, tcp::TcpProxy, Proxy};

pub struct GatewayAgent<Addr> {
    config: Config<Addr>,
    gateways: Vec<Gateway<Addr>>,
}

pub trait Gatewayable<Addr> {
    type GatewayFuture<'cx>: Future<Output = Result<(), GError>>
    where
        Self: 'cx;

    fn new(config: ProxyConfig<Addr>) -> Self;

    fn serve(&self) -> Self::GatewayFuture<'_>;
}

#[derive(Clone)]
pub struct Gateway<Addr> {
    config: ProxyConfig<Addr>,
}

impl Gatewayable<TcpAddress> for Gateway<TcpAddress> {
    type GatewayFuture<'cx> = impl Future<Output = Result<(), GError>> where Self: 'cx;

    fn new(config: ProxyConfig<TcpAddress>) -> Self {
        Self { config }
    }

    fn serve(&self) -> Self::GatewayFuture<'_> {
        async move {
            let mut proxy = TcpProxy::build_with_config(&self.config);
            proxy.io_loop().await
        }
    }
}

impl Gatewayable<Domain> for Gateway<Domain> {
    type GatewayFuture<'cx> = impl Future<Output = Result<(), GError>> where Self: 'cx;

    fn new(config: ProxyConfig<Domain>) -> Self {
        Self { config }
    }

    fn serve<'cx>(&self) -> Self::GatewayFuture<'_> {
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

    fn serve(&mut self) -> Self::Future<'_>;
}

impl GatewayAgentable for GatewayAgent<TcpAddress> {
    type Config = Config<TcpAddress>;

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

impl GatewayAgentable for GatewayAgent<Domain> {
    type Config = Config<Domain>;

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

pub type TcpInBoundConfig = InBoundConfig<TcpAddress>;
pub type TcpOutBoundConfig = OutBoundConfig<TcpAddress>;

pub type HttpInBoundConfig = InBoundConfig<Domain>;
pub type HttpOutBoundConfig = OutBoundConfig<Domain>;

pub type TcpConfig = Config<TcpAddress>;
pub type HttpConfig = Config<Domain>;
