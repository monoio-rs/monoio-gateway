use monoio::net::ListenerConfig;

use crate::{dns::tcp::TcpAddress, gateway::GatewayAgent};

#[derive(Clone)]
pub struct Config {
    pub proxies: Vec<ProxyConfig>,
}

#[derive(Clone)]
pub struct ProxyConfig {
    pub inbound: InBoundConfig,
    pub outbound: OutBoundConfig,
    pub listener: ListenerConfig,
}

#[derive(Clone)]
pub struct InBoundConfig {
    pub server: ServerConfig,
}

#[derive(Clone)]
pub struct OutBoundConfig {
    pub server: ServerConfig,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub addr: TcpAddress,
}

// traits start

// traits ended

impl Config {
    pub fn build(&self) -> GatewayAgent {
        GatewayAgent::build(&self)
    }
}
