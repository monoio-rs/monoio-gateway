use std::collections::HashMap;
use std::future::Future;
use std::io::{self, Cursor};
use std::path::Path;
use std::rc::Rc;

use std::thread;

use anyhow::bail;
use log::info;
use monoio::net::{ListenerConfig, TcpListener};
use monoio_gateway_core::acme::{start_acme, update_certificate, Acmed};
use monoio_gateway_core::config::ProxyConfig;
use monoio_gateway_core::dns::http::Domain;

use monoio_gateway_core::error::GError;
use monoio_gateway_core::http::router::RouterConfig;

use monoio_gateway_core::service::{Service, ServiceBuilder};

use monoio_gateway_services::layer::accept::{Accept, TcpAcceptService};
use monoio_gateway_services::layer::detect::DetectService;
use monoio_gateway_services::layer::router::RouterService;
use monoio_gateway_services::layer::tls::TlsLayer;

use super::Proxy;

pub type HttpProxyConfig = ProxyConfig<Domain>;

pub struct HttpProxy {
    config: Vec<RouterConfig<Domain>>,
}

impl Proxy for HttpProxy {
    type Error = GError;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;

    fn io_loop(&self) -> Self::OutputFuture<'_> {
        async {
            let mut route_map = HashMap::<String, RouterConfig<Domain>>::new();
            for route in self.config.iter() {
                route_map.insert(route.server_name.to_owned(), route.to_owned());
            }
            let route_wrapper = Rc::new(route_map);
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
                let route_cloned = route_wrapper.clone();
                match svc.call(listener_wrapper.clone()).await {
                    Ok(accept) => {
                        monoio::spawn(async move {
                            let mut detect = DetectService::new_http_detect();
                            match detect.call(accept).await {
                                Ok(ty) => match ty {
                                    Some(detect) => {
                                        let (ty, stream, socketaddr) = detect;
                                        let handler = ServiceBuilder::default();
                                        let acc = Accept::from((stream, socketaddr));
                                        match ty {
                                            monoio_gateway_core::http::version::Type::HTTP => {
                                                info!("a http client detected");
                                                let mut handler = handler
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
                                            monoio_gateway_core::http::version::Type::HTTPS => {
                                                info!("a https client detected");
                                                let mut handler = ServiceBuilder::new()
                                                    .layer(TlsLayer::new())
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
                                    }
                                    None => {
                                        log::info!("cannot detect http version");
                                    }
                                },
                                Err(err) => {
                                    log::error!("{}", err)
                                }
                            };
                        });
                        let _detect = DetectService::new_http_detect();
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
    pub fn build_with_config(config: &Vec<RouterConfig<Domain>>) -> Self {
        configure_acme(config);
        Self {
            config: config.clone(),
        }
    }

    pub fn get_listen_port(&self) -> Option<u16> {
        match self.config.first() {
            Some(port) => Some(*port.listen_port.first().unwrap()),
            None => None,
        }
    }
}

/// acme support
fn configure_acme(config: &Vec<RouterConfig<Domain>>) {
    // load local certificate
    for conf in config.iter() {
        if conf.listen_port.contains(&80) {
            continue;
        }
        info!("acme: load {}", conf.server_name);
        if let Some(tls) = &conf.tls {
            let pem_content: io::Result<Vec<u8>>;
            let key_content: io::Result<Vec<u8>>;
            if tls.private_key.is_some() && tls.chain.is_some() {
                // check config private key
                (pem_content, key_content) = (
                    std::fs::read(tls.chain.clone().unwrap()),
                    std::fs::read(tls.private_key.clone().unwrap()),
                );
            } else {
                // check local ssl
                let path = conf.server_name.get_acme_path().unwrap();
                let (pem, key) = (Path::new(&path).join("pem"), Path::new(&path).join("priv"));
                (pem_content, key_content) =
                    (std::fs::read(pem.clone()), std::fs::read(key.clone()));
            }

            if pem_content.is_ok() && key_content.is_ok() {
                info!(
                    "🚀 ssl certificates for {} existed, let's load it.",
                    conf.server_name
                );
                let content = key_content.unwrap();
                update_certificate(
                    conf.server_name.to_owned(),
                    Cursor::new(pem_content.unwrap()),
                    Cursor::new(content),
                );
                info!("🚀 ssl certificates for {} loaded.", conf.server_name);
                continue;
            }
            info!(
                "{} has no local ssl certificate, prepare requesting acme.",
                conf.server_name
            );
            // prepare acme
            let server_name = conf.server_name.to_owned();
            let mail = tls.mail.to_owned();
            thread::spawn(move || {
                monoio::start::<monoio::IoUringDriver, _>(async move {
                    info!(
                        "{} is requesting certificate using email {}",
                        server_name, mail
                    );
                    match start_acme(server_name, mail).await {
                        Err(err) => {
                            log::error!("requesting certificate failed: {}", err);
                        }
                        _ => {}
                    };
                });
            });
        }
    }
}
