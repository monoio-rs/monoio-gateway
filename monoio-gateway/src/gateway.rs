use std::{collections::HashMap, future::Future};

use monoio_gateway_core::{
    config::{Config, InBoundConfig, OutBoundConfig, ProxyConfig},
    dns::{http::Domain, tcp::TcpAddress},
    error::GError,
    http::router::RouterConfig,
    service::{Service, ServiceBuilder},
};
use monoio_gateway_services::layer::{
    accept::TcpAcceptLayer, listen::TcpListenLayer, router::RouterLayer, transfer::TransferService,
};

use crate::proxy::{h1::HttpProxy, tcp::TcpProxy, Proxy};

pub struct GatewayAgent<Addr> {
    config: Vec<RouterConfig<Addr>>,
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
    type Future<'cx>: Future<Output = Result<(), GError>>
    where
        Self: 'cx;

    fn build(config: &Self::Config) -> Self;

    fn serve(&mut self) -> Self::Future<'_>;
}

impl GatewayAgentable for GatewayAgent<TcpAddress> {
    type Config = Vec<RouterConfig<TcpAddress>>;

    type Future<'g> = impl Future<Output = Result<(), anyhow::Error>>
    where
        Self: 'g;

    fn build(config: &Self::Config) -> Self {
        GatewayAgent {
            config: config.clone(),
        }
    }

    fn serve(&'_ mut self) -> Self::Future<'_> {
        async {
            // let mut handlers = vec![];
            // for gw in self.gateways.iter_mut() {
            //     let clone = gw.clone();
            //     let f = monoio::spawn(async move {
            //         match clone.serve().await {
            //             Ok(()) => {}
            //             Err(err) => eprintln!("Error: {}", err),
            //         }
            //     });
            //     handlers.push(f);
            // }
            // for handle in handlers {
            //     let _ = handle.await;
            // }
            Ok(())
        }
    }
}

impl GatewayAgentable for GatewayAgent<Domain> {
    type Config = Vec<RouterConfig<Domain>>;

    type Future<'cx> = impl Future<Output = Result<(), GError>>
    where
        Self: 'cx;

    fn build(config: &Self::Config) -> Self {
        assert!(
            !config.is_empty(),
            "config cannot be empty during building Gateway"
        );
        GatewayAgent {
            config: config.clone(),
        }
    }

    fn serve(&mut self) -> Self::Future<'_> {
        async {
            let mut route_map = HashMap::<String, RouterConfig<Domain>>::new();
            for route in self.config.iter() {
                route_map.insert(route.server_name.to_owned(), route.to_owned());
            }
            let mut svc = ServiceBuilder::default()
                .layer(TcpListenLayer::new_allow_lan(
                    self.get_listen_port().expect("listen port cannot be null"),
                ))
                .layer(TcpAcceptLayer::default())
                .layer(RouterLayer::new(route_map))
                .service(TransferService::default());
            svc.call(()).await?;
            Ok(())
        }
    }
}

impl<A> GatewayAgent<A> {
    pub fn get_listen_port(&self) -> Option<u16> {
        match self.config.first() {
            Some(port) => Some(port.listen_port),
            None => None,
        }
    }
}

pub type TcpInBoundConfig = InBoundConfig<TcpAddress>;
pub type TcpOutBoundConfig = OutBoundConfig<TcpAddress>;

pub type HttpInBoundConfig = InBoundConfig<Domain>;
pub type HttpOutBoundConfig = OutBoundConfig<Domain>;

pub type TcpConfig = Config<TcpAddress>;
pub type HttpConfig = Config<Domain>;
