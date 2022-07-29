use std::{
    future::Future,
    marker::PhantomData,
    net::{SocketAddr, ToSocketAddrs},
};

use monoio::net::ListenerConfig;

use crate::{
    dns::{h1::Domain, tcp::TcpAddress, Resolvable},
    gateway::GatewayAgent,
};

pub type GError = anyhow::Error;

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

#[derive(Clone)]
pub struct OutBoundConfig<'cx, Addr> {
    pub server: ServerConfig<'cx, Addr>,

    pub phantom_data: PhantomData<&'cx Addr>,
}

#[derive(Clone)]
pub struct ServerConfig<'cx, Addr> {
    pub addr: Addr,

    pub phantom_data: PhantomData<&'cx Addr>,
    // TODO: max retries
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

pub type TcpInBoundConfig<'cx> = InBoundConfig<'cx, TcpAddress>;
pub type TcpOutBoundConfig<'cx> = OutBoundConfig<'cx, TcpAddress>;

pub type HttpInBoundConfig<'cx> = InBoundConfig<'cx, Domain>;
pub type HttpOutBoundConfig<'cx> = OutBoundConfig<'cx, Domain>;

pub type TcpConfig<'cx> = Config<'cx, TcpAddress>;
pub type HttpConfig<'cx> = Config<'cx, Domain>;
