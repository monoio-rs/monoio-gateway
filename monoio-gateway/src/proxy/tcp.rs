use std::{future::Future, net::SocketAddr};

use anyhow::Result;
use monoio::net::{TcpListener, TcpStream};

use crate::{config::ProxyConfig, dns::Resolvable, proxy::copy_data};

use super::Proxy;

pub struct TcpProxy {
    config: ProxyConfig,
}

impl Proxy for TcpProxy {
    type Error = anyhow::Error;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>>;

    fn io_loop(&mut self) -> Self::OutputFuture<'_> {
        async {
            // bind inbound port
            let local_addr = self.inbound_addr().await?;
            let peer_addr = self.outbound_addr().await?;
            let listener = TcpListener::bind_with_config(local_addr, &self.config.listener)
                .expect(&format!("cannot bind with address: {}", local_addr));
            // start io loop
            loop {
                let accept = listener.accept().await;
                match accept {
                    Ok((mut conn, _)) => {
                        // async accept logic
                        monoio::spawn(async move {
                            let remote_conn = TcpStream::connect_addr(peer_addr).await;
                            match remote_conn {
                                Ok(mut remote) => {
                                    let (mut local_read, mut local_write) = conn.split();
                                    let (mut remote_read, mut remote_write) = remote.split();
                                    let _ = monoio::join!(
                                        copy_data(&mut local_read, &mut remote_write),
                                        copy_data(&mut remote_read, &mut local_write)
                                    );
                                }
                                Err(_) => {
                                    eprintln!("unable to connect addr: {}", peer_addr)
                                }
                            }
                        });
                    }
                    Err(_) => eprintln!("failed to accept connections."),
                }
            }
        }
    }
}

impl TcpProxy {
    pub fn build_with_config(config: &ProxyConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn inbound_addr(&self) -> Result<SocketAddr> {
        Ok(self.config.inbound.server.addr.resolve().await?)
    }

    pub async fn outbound_addr(&self) -> Result<SocketAddr> {
        Ok(self.config.outbound.server.addr.resolve().await?)
    }
}
