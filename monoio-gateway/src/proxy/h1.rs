use std::collections::HashMap;
use std::future::Future;

use anyhow::bail;

use log::info;
use monoio::net::{ListenerConfig, TcpListener};
use monoio_gateway_core::config::ProxyConfig;
use monoio_gateway_core::dns::http::Domain;

use monoio_gateway_core::error::GError;
use monoio_gateway_core::http::router::RouterConfig;
use monoio_gateway_core::service::{Service, ServiceBuilder};

use monoio_gateway_services::layer::accept::{TcpAccept, TcpAcceptLayer};
use monoio_gateway_services::layer::detect::{DetectResult, DetectService};
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
            let listen_addr = format!("0.0.0.0:{}", self.get_listen_port().unwrap());
            let listener = TcpListener::bind_with_config(listen_addr, &ListenerConfig::default())
                .expect("err bind address");
            let mut svc = ServiceBuilder::default()
                .layer(TcpAcceptLayer::default())
                .service(DetectService::new_http_detect());

            // loop {
            //     match svc.call(()).await {
            //         Ok(ty) => match ty {
            //             Some(detect) => {
            //                 let (ty, stream): DetectResult = detect;
            //                 let mut handler = ServiceBuilder::default();
            //                 match ty {
            //                     monoio_gateway_core::http::version::Type::HTTP => {
            //                         let handler = handler
            //                             .layer(RouterLayer::new(route_map))
            //                             .layer(ConnectEndpointLayer::new())
            //                             .service(HttpTransferService::default());
            //                     }
            //                     monoio_gateway_core::http::version::Type::HTTPS => {
            //                         let handler = ServiceBuilder::new();
            //                     }
            //                 }
            //             }
            //             None => {
            //                 log::info!("cannot detect http version");
            //             }
            //         },
            //         Err(err) => {
            //             log::error!("{}", err)
            //         }
            //     }
            // }

            // let mut svc = svc
            //     .layer(RouterLayer::new(route_map))
            //     .layer(ConnectEndpointLayer::new())
            //     .service(HttpTransferService::default());
            // match svc.call(()).await {
            //     Ok(_) => Ok(()),
            //     Err(err) => bail!("{}", err),
            // }
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
