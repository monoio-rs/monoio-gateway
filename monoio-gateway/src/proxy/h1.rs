use std::{future::Future, net::SocketAddr};

use crate::{
    config::ProxyConfig,
    dns::{http::Domain, Resolvable},
};

use super::Proxy;

pub type HttpProxyConfig<'cx> = ProxyConfig<'cx, Domain>;

pub struct HttpProxy<'cx> {
    config: HttpProxyConfig<'cx>,
}

impl<'cx> Proxy for HttpProxy<'cx> {
    type Error = anyhow::Error;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

    fn io_loop(&mut self) -> Self::OutputFuture<'_> {
        async {
            println!("start a http proxy");
            todo!();
        }
    }
}

impl<'cx> HttpProxy<'cx> {
    pub fn build_with_config(config: &HttpProxyConfig<'cx>) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn inbound_addr(&self) -> Result<SocketAddr, anyhow::Error> {
        let resolved = self.config.inbound.server.addr.resolve().await?;
        if let Some(res) = resolved {
            Ok(res)
        } else {
            Err(anyhow::anyhow!("resolve http inbound addr failed."))
        }
    }

    pub async fn outbound_addr(&self) -> Result<SocketAddr, anyhow::Error> {
        let resolved = self.config.outbound.server.addr.resolve().await?;
        if let Some(res) = resolved {
            Ok(res)
        } else {
            Err(anyhow::anyhow!("resolve http outbound addr failed."))
        }
    }

    pub fn configure(&mut self) {}
}
