#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use std::{marker::PhantomData, net::SocketAddr, str::FromStr, vec};

use anyhow::{Ok, Result};
use monoio::net::ListenerConfig;
use monoio_gateway::{
    config::{Config, ServerConfig, TcpInBoundConfig, TcpOutBoundConfig},
    dns::tcp::TcpAddress,
    gateway::GatewayAgent,
    proxy::tcp::TcpProxyConfig,
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
    let inbound_addr2 = SocketAddr::from_str("127.0.0.1:5001")?;
    let outbound_addr = SocketAddr::from_str("127.0.0.1:8000")?;

    let config = Config {
        proxies: vec![
            TcpProxyConfig {
                inbound: TcpInBoundConfig {
                    server: ServerConfig {
                        addr: TcpAddress {
                            inner: inbound_addr,
                        },
                        phantom_data: PhantomData,
                    },
                    phantom_data: PhantomData,
                },
                outbound: TcpOutBoundConfig {
                    server: ServerConfig {
                        addr: TcpAddress {
                            inner: outbound_addr,
                        },
                        phantom_data: PhantomData,
                    },
                    phantom_data: PhantomData,
                },
                listener: ListenerConfig::default(),
                phantom_data: PhantomData,
            },
            TcpProxyConfig {
                inbound: TcpInBoundConfig {
                    server: ServerConfig {
                        addr: TcpAddress {
                            inner: inbound_addr2,
                        },
                        phantom_data: PhantomData,
                    },
                    phantom_data: PhantomData,
                },
                outbound: TcpOutBoundConfig {
                    server: ServerConfig {
                        addr: TcpAddress {
                            inner: outbound_addr,
                        },
                        phantom_data: PhantomData,
                    },
                    phantom_data: PhantomData,
                },
                listener: ListenerConfig::default(),
                phantom_data: PhantomData,
            },
        ],
        phantom_data: PhantomData,
    };
    let mut agent = GatewayAgent::build(&config);
    agent.serve().await?;
    Ok(())
}
