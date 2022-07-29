use std::{
    future::Future,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
    vec,
};

use anyhow::{Ok, Result};

use crate::{
    config::{Config, GError, ProxyConfig},
    dns::{tcp::TcpAddress, Resolvable},
    proxy::{tcp::TcpProxy, Proxy},
};

pub struct GatewayAgent<'cx, Addr>
where
    Addr: Resolvable<Error = GError> + 'cx,
    Addr::Item<'cx>: ToSocketAddrs + 'cx,
    Addr::ResolveFuture<'cx>: Future<Output = Result<Option<SocketAddr>, GError>>,
{
    config: Config<'cx, Addr>,
    gateways: Vec<Gateway<'cx, Addr>>,

    phantom_data: PhantomData<&'cx Addr>,
}

pub trait Gatewayable<'cx, Addr>
where
    Addr: Resolvable<Error = GError> + 'cx,
    Addr::Item<'cx>: ToSocketAddrs + 'cx,
    Addr::ResolveFuture<'cx>: Future<Output = Result<Option<SocketAddr>, GError>>,
{
    type GatewayFuture: Future<Output = Result<(), GError>>
    where
        Self: 'cx;

    fn new(config: ProxyConfig<'cx, Addr>) -> Self;

    fn serve(&'cx self) -> Self::GatewayFuture;
}

#[derive(Clone)]
pub struct Gateway<'cx, Addr> {
    config: ProxyConfig<'cx, Addr>,

    phantom_data: PhantomData<&'cx Addr>,
}

impl<'cx> Gatewayable<'cx, TcpAddress> for Gateway<'cx, TcpAddress> {
    type GatewayFuture = impl Future<Output = Result<(), GError>> where Self: 'cx;

    fn new(config: ProxyConfig<'cx, TcpAddress>) -> Self {
        Self {
            config,
            phantom_data: PhantomData,
        }
    }

    fn serve(&'cx self) -> Self::GatewayFuture {
        async move {
            let mut proxy = TcpProxy::build_with_config(&self.config);
            proxy.io_loop().await;
            Ok(())
        }
    }
}
impl<'cx> GatewayAgent<'cx, TcpAddress>
where
    'cx: 'static,
{
    pub fn build(config: &Config<'cx, TcpAddress>) -> Self {
        let gateways: Vec<Gateway<TcpAddress>> = config
            .proxies
            .iter()
            .map(|proxy_config| Gateway::new(proxy_config.clone()))
            .collect();
        GatewayAgent {
            config: config.clone(),
            gateways,
            phantom_data: PhantomData,
        }
    }

    /// serve current gateway, ensure all gateways
    async fn _serve(&mut self) -> Result<()> {
        let mut gws = self.gateways.clone();
        let mut handlers = vec![];
        for gw in gws.iter_mut() {
            let clone = gw.clone();
            let f = monoio::spawn(async move { clone.serve().await });
            handlers.push(f);
        }
        for handle in handlers {
            let _ = handle.await;
        }
        Ok(())
    }

    pub async fn serve(&mut self) -> Result<()> {
        self._serve().await?;
        Ok(())
    }
}
