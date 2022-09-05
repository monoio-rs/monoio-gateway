use std::collections::HashMap;
use std::future::Future;
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;

use std::thread;

use log::info;
use monoio::net::{ListenerConfig, TcpListener};
use monoio_gateway_core::acme::{start_acme, update_certificate, Acmed};
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
                                    info!("a http client detected");
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
                                    info!("a https client detected");
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
        info!("acme: load {}", conf.server_name);
        if let Some(tls) = &conf.tls {
            if tls.private_key.is_some() || tls.root_ca.is_some() || tls.server_key.is_some() {
                continue;
            }
            // check local ssl
            let path = conf.server_name.get_acme_path().unwrap();
            let (pem, key) = (Path::new(&path).join("pem"), Path::new(&path).join("priv"));
            let (pem_content, key_content) =
                (std::fs::read(pem.clone()), std::fs::read(key.clone()));

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
                monoio::start::<monoio::LegacyDriver, _>(async move {
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
