#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use std::{net::SocketAddr, str::FromStr, vec};

use anyhow::{Ok, Result};
use monoio::net::ListenerConfig;
use monoio_gateway::{
    config::{Config, InBoundConfig, OutBoundConfig, ProxyConfig, ServerConfig},
    dns::tcp::TcpAddress,
};

pub mod balance;
pub mod config;
pub mod discover;
pub mod dns;
pub mod gateway;
pub mod http;
pub mod layer;
pub mod proxy;

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<()> {
    let inbound_addr = SocketAddr::from_str("127.0.0.1:5000")?;
    let outbound_addr = SocketAddr::from_str("127.0.0.1:9999")?;

    let config = Config {
        proxies: vec![ProxyConfig {
            inbound: InBoundConfig {
                server: ServerConfig {
                    addr: TcpAddress::new(inbound_addr),
                },
            },
            outbound: OutBoundConfig {
                server: ServerConfig {
                    addr: TcpAddress::new(outbound_addr),
                },
            },
            listener: ListenerConfig::default(),
        }],
    };

    let mut agent = config.build();
    agent.serve().await?;

    Ok(())
}
