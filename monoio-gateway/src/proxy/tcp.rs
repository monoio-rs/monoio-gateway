use std::{future::Future, net::SocketAddr};

use monoio::net::{TcpListener, TcpStream};

use crate::{
    config::ProxyConfig,
    dns::{tcp::TcpAddress, Resolvable},
    proxy::copy_data,
};

use super::Proxy;

pub type TcpProxyConfig<'cx> = ProxyConfig<'cx, TcpAddress>;

pub struct TcpProxy<'cx> {
    config: TcpProxyConfig<'cx>,
}

impl<'cx> Proxy for TcpProxy<'cx> {
    type Error = anyhow::Error;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

    fn io_loop(&mut self) -> Self::OutputFuture<'_> {
        async {
            println!("starting a new tcp proxy");
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

impl<'cx> TcpProxy<'cx> {
    pub fn build_with_config(config: &TcpProxyConfig<'cx>) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn inbound_addr(&self) -> Result<SocketAddr, anyhow::Error> {
        let resolved = self.config.inbound.server.addr.resolve().await?;
        if let Some(res) = resolved {
            Ok(res)
        } else {
            Err(anyhow::anyhow!("resolve tcp inbound addr failed."))
        }
    }

    pub async fn outbound_addr(&self) -> Result<SocketAddr, anyhow::Error> {
        let resolved = self.config.outbound.server.addr.resolve().await?;
        if let Some(res) = resolved {
            Ok(res)
        } else {
            Err(anyhow::anyhow!("resolve tcp inbound addr failed."))
        }
    }

    pub fn configure(&mut self) {}
}
