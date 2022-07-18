use std::net::SocketAddr;

use monoio::net::ListenerConfig;

use crate::gateway::GatewayAgent;

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
    pub addr: SocketAddr,
}

#[derive(Clone)]
pub struct OutBoundConfig {
    pub addr: SocketAddr,
}

// traits start

// traits ended

impl Config {
    pub fn build(&self) -> GatewayAgent {
        GatewayAgent::build(&self)
    }
}
