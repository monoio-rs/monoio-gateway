use std::{future::Future, net::SocketAddr, str::FromStr};

use monoio::{
    io::Splitable,
    net::{ListenerConfig, TcpListener, TcpStream},
};
use monoio_gateway_core::{
    dns::tcp::TcpAddress, error::GError, http::router::RouterConfig, transfer::copy_data,
};

use super::Proxy;

pub type TcpProxyConfig = RouterConfig<TcpAddress>;

pub struct TcpProxy {
    config: TcpProxyConfig,
    listener_config: ListenerConfig,
}

impl Proxy for TcpProxy {
    type Error = anyhow::Error;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;

    fn io_loop(&self) -> Self::OutputFuture<'_> {
        async {
            println!("starting a new tcp proxy");
            // bind inbound port
            let local_addr = self.inbound_addr().await?;
            let peer_addr = self.outbound_addr().await?;
            let listener = TcpListener::bind_with_config(local_addr, &self.listener_config)
                .expect(&format!("cannot bind with address: {}", local_addr));
            // start io loop
            loop {
                let accept = listener.accept().await;
                match accept {
                    Ok((mut conn, _)) => {
                        // async accept logic
                        monoio::spawn(async move {
                            let remote_conn = TcpStream::connect(peer_addr).await;
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
    pub fn build_with_config(config: &Vec<TcpProxyConfig>) -> Self {
        assert!(config.len() == 1, "tcp proxy can only have one endpoint!");
        Self {
            config: config.first().unwrap().to_owned(),
            listener_config: ListenerConfig::default(),
        }
    }

    #[inline]
    pub async fn inbound_addr(&self) -> Result<TcpAddress, GError> {
        let addr = format!("0.0.0.0:{}", self.config.listen_port.first().unwrap());
        Ok(TcpAddress::new(
            SocketAddr::from_str(&addr).expect(&format!("addr {} is not valid", addr)),
        ))
    }

    #[inline]
    pub async fn outbound_addr(&self) -> Result<TcpAddress, GError> {
        Ok(self.config.rules.first().unwrap().proxy_pass)
    }

    pub fn configure(&mut self) {}
}
