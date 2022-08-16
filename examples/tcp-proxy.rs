use std::{net::SocketAddr, str::FromStr};

use monoio::net::ListenerConfig;
use monoio_gateway::{
    gateway::{GatewayAgentable, TcpInBoundConfig, TcpOutBoundConfig},
    proxy::tcp::TcpProxyConfig,
};
use monoio_gateway_core::{
    config::{Config, ServerConfig},
    dns::tcp::TcpAddress,
};

/// a simple tcp proxy
#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), anyhow::Error> {
    let inbound_addr = SocketAddr::from_str("127.0.0.1:5000")?;
    let inbound_addr2 = SocketAddr::from_str("127.0.0.1:5001")?;
    let outbound_addr = SocketAddr::from_str("127.0.0.1:8000")?;
    let _config = Config::new()
        .push(TcpProxyConfig {
            inbound: TcpInBoundConfig::new(ServerConfig::new(TcpAddress::new(inbound_addr))),
            outbound: TcpOutBoundConfig::new(ServerConfig::new(TcpAddress::new(
                outbound_addr.clone(),
            ))),
            listener: ListenerConfig::default(),
        })
        .push(TcpProxyConfig {
            inbound: TcpInBoundConfig::new(ServerConfig::new(TcpAddress::new(inbound_addr2))),
            outbound: TcpOutBoundConfig::new(ServerConfig::new(TcpAddress::new(
                outbound_addr.clone(),
            ))),
            listener: ListenerConfig::default(),
        });
    // let mut agent = GatewayAgent::<TcpAddress>::build(&config);
    // agent.serve().await?;
    Ok(())
}
