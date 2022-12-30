use std::future::Future;
use std::rc::Rc;

use std::time::Duration;

use anyhow::{bail, Result};
use log::info;
use monoio::net::{ListenerConfig, TcpListener};

use monoio_gateway_core::{
    config::ProxyConfig,
    dns::http::Domain,
    error::GError,
    http::router::RouterConfig,
    service::{Service, ServiceBuilder},
};

use monoio_gateway_services::layer::timeout::TimeoutLayer;
use monoio_gateway_services::layer::{
    accept::TcpAcceptService, router::RouterService, tls::TlsLayer,
};

use super::Proxy;

pub type HttpProxyConfig = ProxyConfig<Domain>;

pub struct HttpProxy {
    config: RouterConfig<Domain>,
    port: u16,
}

impl Proxy for HttpProxy {
    type Error = GError;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;

    fn io_loop(&self) -> Self::OutputFuture<'_> {
        async {
            let routes = Rc::new(self.config.to_owned());
            let ty = self.config.protocol;
            let listen_addr = format!("0.0.0.0:{}", self.get_listen_port().unwrap());
            let listener = TcpListener::bind_with_config(listen_addr, &ListenerConfig::default());
            if let Err(e) = listener {
                bail!("Error when binding address({})", e);
            }
            let listener = listener.unwrap();
            let listener_wrapper = Rc::new(listener);
            let mut svc = ServiceBuilder::default().service(TcpAcceptService::default());
            loop {
                log::info!(
                    "📈 new accept avaliable for {:?}, waiting",
                    self.get_listen_port()
                );
                let server_name = self.config.server_name.clone();
                let route_cloned = routes.clone();
                match svc.call(listener_wrapper.clone()).await {
                    Ok(accept) => {
                        monoio::spawn(async move {
                            let handler = ServiceBuilder::default();
                            let acc = accept;
                            match ty {
                                monoio_gateway_core::http::version::Type::HTTP => {
                                    info!("a http client detected");
                                    let mut handler =
                                        handler.service(RouterService::new(route_cloned));
                                    match handler.call(acc).await {
                                        Ok(_) => {
                                            info!("✔ complete connection");
                                        }
                                        Err(e) => {
                                            log::error!("{}", e);
                                        }
                                    }
                                }
                                monoio_gateway_core::http::version::Type::HTTPS => {
                                    info!("a https client detected");
                                    let mut handler = ServiceBuilder::new()
                                        .layer(TlsLayer::new(server_name))
                                        .layer(TimeoutLayer::new(Duration::from_secs(10)))
                                        .service(RouterService::new(route_cloned));
                                    match handler.call(acc).await {
                                        Ok(_) => {
                                            info!("✔ complete connection");
                                        }
                                        Err(e) => {
                                            log::error!("{}", e);
                                        }
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        log::warn!("tcp accept failed: {}", e);
                    }
                }
            }
        }
    }
}

impl HttpProxy {
    pub fn build_with_config(config: &RouterConfig<Domain>, port: u16) -> Self {
        Self {
            config: config.clone(),
            port,
        }
    }

    pub fn get_listen_port(&self) -> Option<u16> {
        Some(self.port)
    }
}
