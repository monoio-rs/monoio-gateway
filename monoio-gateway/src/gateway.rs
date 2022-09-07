use std::{future::Future, thread};

use log::info;
use monoio::RuntimeBuilder;
use monoio_gateway_core::{
    config::{Config, InBoundConfig, OutBoundConfig},
    dns::{http::Domain, tcp::TcpAddress, Resolvable},
    error::GError,
    http::router::{Router, RouterConfig},
};
use monoio_http::ParamRef;

use crate::proxy::{h1::HttpProxy, tcp::TcpProxy, Proxy};

pub trait Gatewayable<Addr> {
    type GatewayFuture<'cx>: Future<Output = Result<(), GError>>
    where
        Self: 'cx;

    fn new(config: Vec<RouterConfig<Addr>>) -> Self;

    fn from_router(router: Router<Addr>) -> Vec<Gateway<Addr>>;

    fn serve(&self) -> Self::GatewayFuture<'_>;
}

#[derive(Clone)]
pub struct Gateway<Addr> {
    config: Vec<RouterConfig<Addr>>,
}

impl Gatewayable<TcpAddress> for Gateway<TcpAddress> {
    type GatewayFuture<'cx> = impl Future<Output = Result<(), GError>> where Self: 'cx;

    fn new(config: Vec<RouterConfig<TcpAddress>>) -> Self {
        Self { config }
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
            info!("port: {}, gateway payload count: {}", port, v.len());
            let config_vec = v.clone();
            agent_vec.push(Gateway::new(config_vec));
        }
        agent_vec
    }
}

impl Gatewayable<Domain> for Gateway<Domain> {
    type GatewayFuture<'cx> = impl Future<Output = Result<(), GError>> where Self: 'cx;

    fn new(config: Vec<RouterConfig<Domain>>) -> Self {
        Self { config }
    }

    fn serve<'cx>(&self) -> Self::GatewayFuture<'_> {
        async move {
            let proxy = HttpProxy::build_with_config(&self.config);
            proxy.io_loop().await?;
            Ok(())
        }
    }

    fn from_router(router: Router<Domain>) -> Vec<Gateway<Domain>> {
        let m = router.param_ref();
        info!("starting {} services", m.len());
        let mut agent_vec = vec![];
        for (port, v) in m {
            info!("port: {}, gateway payload count: {}", port, v.len());
            let config_vec = v.clone();
            agent_vec.push(Gateway::new(config_vec));
        }
        agent_vec
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
    type Future<'a> = impl Future<Output = Result<(), GError>>
     where Self: 'a;

    fn serve(&self) -> Self::Future<'_> {
        async {
            let mut handler_vec = vec![];
            for gw in self.iter() {
                let cloned = gw.clone();
                let handler = thread::spawn(move || {
                    let mut rt = RuntimeBuilder::<monoio::IoUringDriver>::new()
                        .enable_timer()
                        .with_entries(32768)
                        .build()
                        .unwrap();
                    rt.block_on(async move {
                        match cloned.serve().await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("Gateway Error: {}", err);
                            }
                        }
                    });
                });
                handler_vec.push(handler);
            }
            // wait to exit
            for handle in handler_vec.into_iter() {
                let _ = handle.join();
            }
            Ok(())
        }
    }
}
