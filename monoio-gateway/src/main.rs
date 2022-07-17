use std::{vec, net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4}};

use anyhow::{Result, Ok};
use monoio::net::ListenerConfig;
use monoio_gateway::config::{Config, ProxyConfig, InBoundConfig, OutBoundConfig};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<()> {
    let local_addr = Ipv4Addr::new(127, 0, 0, 1);
    let inbound_addr = SocketAddr::V4(SocketAddrV4::new(local_addr.clone(), 5000));
    let outbound_addr = SocketAddr::V4(SocketAddrV4::new(local_addr.clone(), 9999));

    let config = Config{
        proxies: vec![
            ProxyConfig {
                inbound: InBoundConfig { addr: inbound_addr },
                outbound: OutBoundConfig {addr: outbound_addr },
            }
        ],
        listener: ListenerConfig::default(),
    };

    let mut agent = config.build();
    agent.serve().await?;

    Ok(())
}