use std::collections::HashMap;
use std::future::Future;

use anyhow::bail;

use monoio_gateway_core::config::ProxyConfig;
use monoio_gateway_core::dns::http::Domain;

use monoio_gateway_core::error::GError;
use monoio_gateway_core::http::router::RouterConfig;
use monoio_gateway_core::service::{Service, ServiceBuilder};

use monoio_gateway_services::layer::accept::TcpAcceptLayer;
use monoio_gateway_services::layer::endpoint::ConnectEndpointLayer;
use monoio_gateway_services::layer::listen::TcpListenLayer;
use monoio_gateway_services::layer::router::RouterLayer;
use monoio_gateway_services::layer::transfer::HttpTransferService;

use super::Proxy;

pub type HttpProxyConfig = ProxyConfig<Domain>;

pub struct HttpProxy {
    config: Vec<RouterConfig<Domain>>,
}

impl Proxy for HttpProxy {
    type Error = GError;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

    fn io_loop(&mut self) -> Self::OutputFuture<'_> {
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
                .layer(ConnectEndpointLayer::new())
                .service(HttpTransferService::default());
            match svc.call(()).await {
                Ok(_) => Ok(()),
                Err(err) => bail!("{}", err),
            }
        }
    }
}

impl HttpProxy {
    pub fn build_with_config(config: &Vec<RouterConfig<Domain>>) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn get_listen_port(&self) -> Option<u16> {
        match self.config.first() {
            Some(port) => Some(port.listen_port),
            None => None,
        }
    }

    pub fn configure(&mut self) {}
}
