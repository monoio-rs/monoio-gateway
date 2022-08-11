use std::vec;

use monoio::net::ListenerConfig;

#[derive(Clone)]
pub struct Config<Addr> {
    pub proxies: Vec<ProxyConfig<Addr>>,
}

impl<Addr> Config<Addr> {
    pub fn new() -> Self {
        Self { proxies: vec![] }
    }

    pub fn push(mut self, proxy_config: ProxyConfig<Addr>) -> Self {
        self.proxies.push(proxy_config);
        self
    }
}

#[derive(Clone)]
pub struct ProxyConfig<Addr> {
    pub inbound: InBoundConfig<Addr>,
    pub outbound: OutBoundConfig<Addr>,
    pub listener: ListenerConfig,
}

#[derive(Clone)]
pub struct InBoundConfig<Addr> {
    pub server: ServerConfig<Addr>,
}

impl<Addr> InBoundConfig<Addr> {
    pub fn new(config: ServerConfig<Addr>) -> Self {
        Self { server: config }
    }
}

#[derive(Clone)]
pub struct OutBoundConfig<Addr> {
    pub server: ServerConfig<Addr>,
}

impl<Addr> OutBoundConfig<Addr> {
    pub fn new(config: ServerConfig<Addr>) -> Self {
        Self { server: config }
    }
}

#[derive(Clone)]
pub struct ServerConfig<Addr> {
    pub addr: Addr,
    // TODO: max retries
}

impl<Addr> ServerConfig<Addr> {
    pub fn new(addr: Addr) -> Self {
        Self { addr }
    }
}

// traits start

// traits ended
