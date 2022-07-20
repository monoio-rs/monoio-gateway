use std::vec;

use anyhow::{Ok, Result};

use crate::{
    config::{Config, ProxyConfig},
    proxy::{tcp::TcpProxy, Proxy},
};

pub struct GatewayAgent {
    config: Config,
    gateways: Vec<Gateway>,
}

#[derive(Clone)]
pub struct Gateway {
    config: ProxyConfig,
}

impl Gateway {
    pub fn new(config: &ProxyConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// serve current gateway
    pub async fn serve(&mut self) -> Result<()> {
        let _inbound = &self.config.inbound;
        // server with pure TCP
        // TODO: UDP
        self.legacy_serve().await
    }

    pub async fn legacy_serve(&mut self) -> Result<()> {
        let mut proxy = TcpProxy::build_with_config(&self.config);
        proxy.io_loop().await
    }
}

impl GatewayAgent {
    pub fn build(config: &Config) -> Self {
        let gateways: Vec<Gateway> = config
            .proxies
            .iter()
            .map(|proxy_config| Gateway::new(proxy_config))
            .collect();
        GatewayAgent {
            config: config.clone(),
            gateways,
        }
    }

    /// serve current gateway, ensure all gateways
    async fn _serve(&mut self) -> Result<()> {
        let mut handlers = vec![];
        for gw in self.gateways.iter_mut() {
            let mut clone = gw.clone();
            let t = monoio::spawn(async move {
                let _ = clone.serve().await;
            });
            handlers.push(t);
        }
        for handle in handlers {
            handle.await;
        }
        Ok(())
    }

    pub async fn serve(&mut self) -> Result<()> {
        self._serve().await?;
        Ok(())
    }
}
