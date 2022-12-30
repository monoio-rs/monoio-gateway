use std::{future::Future, io::Cursor};

use anyhow::{bail, Result};
use log::{debug, info};

use monoio_gateway_core::{
    acme::update_certificate,
    config::{Config, InBoundConfig, OutBoundConfig},
    dns::{http::Domain, tcp::TcpAddress, Resolvable},
    error::GError,
    http::{
        router::{Router, RouterConfig},
        version::Type,
    },
};
use monoio_http::ParamRef;

use crate::proxy::{h1::HttpProxy, tcp::TcpProxy, Proxy};

pub trait Gatewayable<Addr> {
    type GatewayFuture<'cx>: Future<Output = Result<(), GError>>
    where
        Self: 'cx;

    fn new(config: RouterConfig<Addr>, port: u16) -> Self;

    fn from_router(router: Router<Addr>) -> Vec<Gateway<Addr>>;

    fn serve(&self) -> Self::GatewayFuture<'_>;
}

#[derive(Clone)]
pub struct Gateway<Addr> {
    config: RouterConfig<Addr>,
    port: u16,
}

impl Gatewayable<TcpAddress> for Gateway<TcpAddress> {
    type GatewayFuture<'cx> = impl Future<Output = Result<(), GError>> + 'cx where Self: 'cx;

    fn new(config: RouterConfig<TcpAddress>, port: u16) -> Self {
        Self { config, port }
    }

    fn serve(&self) -> Self::GatewayFuture<'_> {
        async move {
            let proxy = TcpProxy::build_with_config(&self.config);
            proxy.io_loop().await
        }
    }

    fn from_router(router: Router<TcpAddress>) -> Vec<Gateway<TcpAddress>>
    where
        Self: Sized,
    {
        let m = router.param_ref();
        info!("starting {} services", m.len());
        let mut agent_vec = vec![];
        for (port, v) in m {
            info!("gateway port: {}", port);
            let config_vec = v.clone();
            agent_vec.push(Gateway::new(config_vec, *port));
        }
        agent_vec
    }
}

impl Gatewayable<Domain> for Gateway<Domain> {
    type GatewayFuture<'cx> = impl Future<Output = Result<(), GError>> + 'cx where Self: 'cx;

    fn new(config: RouterConfig<Domain>, port: u16) -> Self {
        Self { config, port }
    }

    fn serve<'cx>(&self) -> Self::GatewayFuture<'_> {
        async move {
            self.configure_cert()?;
            let proxy = HttpProxy::build_with_config(&self.config, self.port);
            proxy.io_loop().await?;
            Ok(())
        }
    }

    fn from_router(router: Router<Domain>) -> Vec<Gateway<Domain>> {
        let m = router.param_ref();
        info!("starting {} services", m.len());
        let mut agent_vec = vec![];
        for (port, v) in m {
            info!("gateway port: {}", port);
            let config = v.clone();
            agent_vec.push(Gateway::new(config, *port));
        }
        agent_vec
    }
}

impl Gateway<Domain> {
    fn configure_cert(&self) -> Result<()> {
        let config = &self.config;
        if config.protocol != Type::HTTPS {
            debug!("non https server, ignore cert config");
            return Ok(());
        }
        if let Some(tls) = &config.tls {
            if tls.private_key.is_none() || tls.chain.is_none() {
                bail!(
                    "server: {}, private key provided: {}, certificate chain provided: {}",
                    config.server_name,
                    tls.private_key.is_none(),
                    tls.chain.is_none()
                );
            }

            let (pem_content, key_content) = (
                std::fs::read(tls.chain.clone().unwrap()),
                std::fs::read(tls.private_key.clone().unwrap()),
            );

            if pem_content.is_err() || key_content.is_err() {
                bail!(
                    "server: {}, private key read error: {}, certificate chain read error: {}",
                    config.server_name,
                    key_content.is_err(),
                    pem_content.is_err()
                );
            }

            update_certificate(
                config.server_name.to_owned(),
                Cursor::new(pem_content.unwrap()),
                Cursor::new(key_content.unwrap()),
            );

            info!("🚀 ssl certificates for {} loaded.", config.server_name);

            Ok(())
        } else {
            bail!("https server without cert config");
        }
    }
}

pub type TcpInBoundConfig = InBoundConfig<TcpAddress>;
pub type TcpOutBoundConfig = OutBoundConfig<TcpAddress>;

pub type HttpInBoundConfig = InBoundConfig<Domain>;
pub type HttpOutBoundConfig = OutBoundConfig<Domain>;

pub type TcpConfig = Config<TcpAddress>;
pub type HttpConfig = Config<Domain>;

pub trait Servable {
    type Future<'a>: Future<Output = Result<(), GError>>
    where
        Self: 'a;

    fn serve(&self) -> Self::Future<'_>;
}

impl<A> Servable for Vec<Gateway<A>>
where
    A: Resolvable + Send + 'static,
    Gateway<A>: Gatewayable<A>,
{
    type Future<'a> = impl Future<Output = Result<(), GError>> + 'a
     where Self: 'a;

    fn serve(&self) -> Self::Future<'_> {
        async {
            let mut handler_vec = vec![];
            for gw in self.iter() {
                let cloned = gw.clone();
                let handler = monoio::spawn(async move {
                    match cloned.serve().await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Gateway Error: {}", err);
                        }
                    }
                });
                handler_vec.push(handler);
            }
            // wait to exit
            for handle in handler_vec.into_iter() {
                let _ = handle.await;
            }
            Ok(())
        }
    }
}
