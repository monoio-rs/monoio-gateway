use std::{net::SocketAddr};

use anyhow::{Result, Ok};

use crate::config::{Config, self, ProxyConfig};


pub struct GatewayAgent {
    config: Config,
    gateways: Vec<Gateway>
}

#[derive(Clone)]
pub struct Gateway {
    config: ProxyConfig,
}

impl Gateway {
    
    pub fn new(config: &ProxyConfig) -> Self {
        Self { config: config.clone() }
    }

    /// serve current gateway
    pub async fn serve(&mut self) {

    }
}


impl GatewayAgent {
    pub fn build(config: &Config) -> Self {
        let gateways: Vec<Gateway> = config.proxies.iter().map(|proxy_config|{
            Gateway::new(proxy_config)
        }).collect();
        GatewayAgent {
            config: config.clone(),
            gateways
        }
    }

    /// serve current gateway, ensure all gateways
    async fn _serve(&mut self) -> Result<()> {
        for gw in self.gateways.iter_mut() {
            let mut clone = gw.clone();
            monoio::spawn(async move {
                clone.serve().await;
            });
        }
        Ok(())
    }

    pub async fn serve(&mut self) -> Result<()>{
        self._serve().await?;
        Ok(())
    }
}