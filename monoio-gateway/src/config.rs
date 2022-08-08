use std::{
    future::Future,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
    vec,
};

use monoio::net::ListenerConfig;
use monoio_gateway_core::error::GError;

use crate::{
    dns::{http::Domain, tcp::TcpAddress, Resolvable},
    gateway::{GatewayAgent, GatewayAgentable},
};

#[derive(Clone)]
pub struct Config<'cx, Addr>
where
    Addr: Resolvable<Error = GError> + 'cx,
    Addr::Item<'cx>: ToSocketAddrs + 'cx,
    Addr::ResolveFuture<'cx>: Future<Output = Result<Option<SocketAddr>, GError>>,
{
    pub proxies: Vec<ProxyConfig<'cx, Addr>>,

    pub phantom_data: PhantomData<&'cx Addr>,
}

impl<'cx, Addr> Config<'cx, Addr>
where
    Addr: Resolvable<Error = GError> + 'cx,
    Addr::Item<'cx>: ToSocketAddrs + 'cx,
    Addr::ResolveFuture<'cx>: Future<Output = Result<Option<SocketAddr>, GError>>,
{
    pub fn new() -> Self {
        Self {
            proxies: vec![],
            phantom_data: PhantomData,
        }
    }

    pub fn push(mut self, proxy_config: ProxyConfig<'cx, Addr>) -> Self {
        self.proxies.push(proxy_config);
        self
    }
}

#[derive(Clone)]
pub struct ProxyConfig<'cx, Addr> {
    pub inbound: InBoundConfig<'cx, Addr>,
    pub outbound: OutBoundConfig<'cx, Addr>,
    pub listener: ListenerConfig,

    pub phantom_data: PhantomData<&'cx Addr>,
}

#[derive(Clone)]
pub struct InBoundConfig<'cx, Addr> {
    pub server: ServerConfig<'cx, Addr>,

    pub phantom_data: PhantomData<&'cx Addr>,
}

impl<'cx, Addr> InBoundConfig<'cx, Addr> {
    pub fn new(config: ServerConfig<'cx, Addr>) -> Self {
        Self {
            server: config,
            phantom_data: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct OutBoundConfig<'cx, Addr> {
    pub server: ServerConfig<'cx, Addr>,

    pub phantom_data: PhantomData<&'cx Addr>,
}

impl<'cx, Addr> OutBoundConfig<'cx, Addr> {
    pub fn new(config: ServerConfig<'cx, Addr>) -> Self {
        Self {
            server: config,
            phantom_data: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct ServerConfig<'cx, Addr> {
    pub addr: Addr,

    pub phantom_data: PhantomData<&'cx Addr>,
    // TODO: max retries
}

impl<'cx, Addr> ServerConfig<'cx, Addr> {
    pub fn new(addr: Addr) -> Self {
        Self {
            addr,
            phantom_data: PhantomData,
        }
    }
}

// traits start

// traits ended

impl<'cx> Config<'cx, TcpAddress>
where
    'cx: 'static,
{
    pub fn build(&self) -> GatewayAgent<'cx, TcpAddress> {
        GatewayAgent::build(self)
    }
}

impl<'cx> Config<'cx, Domain>
where
    'cx: 'static,
{
    pub fn build(&self) -> GatewayAgent<'cx, Domain> {
        GatewayAgent::build(self)
    }
}

pub type TcpInBoundConfig<'cx> = InBoundConfig<'cx, TcpAddress>;
pub type TcpOutBoundConfig<'cx> = OutBoundConfig<'cx, TcpAddress>;

pub type HttpInBoundConfig<'cx> = InBoundConfig<'cx, Domain>;
pub type HttpOutBoundConfig<'cx> = OutBoundConfig<'cx, Domain>;

pub type TcpConfig<'cx> = Config<'cx, TcpAddress>;
pub type HttpConfig<'cx> = Config<'cx, Domain>;
