use std::{marker::PhantomData, net::SocketAddr, str::FromStr};

use monoio::net::ListenerConfig;
use monoio_gateway::{
    config::{Config, ServerConfig, TcpInBoundConfig, TcpOutBoundConfig},
    dns::tcp::TcpAddress,
    gateway::GatewayAgentable,
    proxy::tcp::TcpProxyConfig,
};

/// a simple tcp proxy
#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), anyhow::Error> {
    let inbound_addr = SocketAddr::from_str("127.0.0.1:5000")?;
    let inbound_addr2 = SocketAddr::from_str("127.0.0.1:5001")?;
    let outbound_addr = SocketAddr::from_str("127.0.0.1:8000")?;
    let config = Config::new()
        .push(TcpProxyConfig {
            inbound: TcpInBoundConfig::new(ServerConfig::new(TcpAddress::new(inbound_addr))),
            outbound: TcpOutBoundConfig::new(ServerConfig::new(TcpAddress::new(
                outbound_addr.clone(),
            ))),
            listener: ListenerConfig::default(),
            phantom_data: PhantomData,
        })
        .push(TcpProxyConfig {
            inbound: TcpInBoundConfig::new(ServerConfig::new(TcpAddress::new(inbound_addr2))),
            outbound: TcpOutBoundConfig::new(ServerConfig::new(TcpAddress::new(
                outbound_addr.clone(),
            ))),
            listener: ListenerConfig::default(),
            phantom_data: PhantomData,
        });
    let mut agent = config.build();
    agent.serve().await?;
    Ok(())
}
