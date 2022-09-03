use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;

use monoio::net::{ListenerConfig, TcpListener};
use monoio_gateway_core::acme::start_acme;
use monoio_gateway_core::config::ProxyConfig;
use monoio_gateway_core::dns::http::Domain;

use monoio_gateway_core::error::GError;
use monoio_gateway_core::http::router::RouterConfig;
use monoio_gateway_core::service::{Service, ServiceBuilder};

use monoio_gateway_services::layer::accept::{Accept, TcpAcceptLayer};
use monoio_gateway_services::layer::detect::DetectService;
use monoio_gateway_services::layer::endpoint::ConnectEndpointLayer;

use monoio_gateway_services::layer::router::RouterLayer;
use monoio_gateway_services::layer::tls::TlsLayer;
use monoio_gateway_services::layer::transfer::HttpTransferService;

use super::Proxy;

pub type HttpProxyConfig = ProxyConfig<Domain>;

pub struct HttpProxy {
    config: Vec<RouterConfig<Domain>>,
}

impl Proxy for HttpProxy {
    type Error = GError;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

    fn io_loop(&self) -> Self::OutputFuture<'_> {
        async {
            let mut route_map = HashMap::<String, RouterConfig<Domain>>::new();
            for route in self.config.iter() {
                route_map.insert(route.server_name.to_owned(), route.to_owned());
            }
            let route_wrapper = Rc::new(route_map);
            let listen_addr = format!("0.0.0.0:{}", self.get_listen_port().unwrap());
            let listener = TcpListener::bind_with_config(listen_addr, &ListenerConfig::default())
                .expect("err bind address");
            let listener_wrapper = Rc::new(listener);
            let mut svc = ServiceBuilder::default()
                .layer(TcpAcceptLayer::default())
                .service(DetectService::new_http_detect());
            loop {
                match svc.call(listener_wrapper.clone()).await {
                    Ok(ty) => match ty {
                        Some(detect) => {
                            let (ty, stream, socketaddr) = detect;
                            let handler = ServiceBuilder::default();
                            let acc = Accept::from((stream, socketaddr));
                            match ty {
                                monoio_gateway_core::http::version::Type::HTTP => {
                                    let mut handler = handler
                                        .layer(RouterLayer::new(route_wrapper.clone()))
                                        .layer(ConnectEndpointLayer::new())
                                        .service(HttpTransferService::default());
                                    match handler.call(acc).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            log::error!("{}", e);
                                            continue;
                                        }
                                    }
                                }
                                monoio_gateway_core::http::version::Type::HTTPS => {
                                    let mut handler = ServiceBuilder::new()
                                        .layer(TlsLayer::new())
                                        .layer(RouterLayer::new(route_wrapper.clone()))
                                        .layer(ConnectEndpointLayer::new())
                                        .service(HttpTransferService::default());
                                    match handler.call(acc).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            log::error!("{}", e);
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            log::info!("cannot detect http version");
                        }
                    },
                    Err(err) => {
                        log::error!("{}", err)
                    }
                }
            }

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

    /// acme support
    pub async fn configure_acme(&self) {
        for conf in self.config.iter() {
            if let Some(tls) = &conf.tls {
                if tls.private_key.is_none() || tls.root_ca.is_none() || tls.server_key.is_none() {
                    continue;
                }
                let server_name = conf.server_name.to_owned();
                let mail = tls.mail.to_owned();
                monoio::spawn(async move {
                    let _ = start_acme(server_name, mail).await;
                });
            }
        }
    }
}
