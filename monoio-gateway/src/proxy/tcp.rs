use anyhow::{Ok, Result};
use monoio::net::{ListenerConfig, TcpListener, TcpStream};

use crate::{
    config::{InBoundConfig, OutBoundConfig, ProxyConfig},
    proxy::copy_data,
};

pub struct TcpProxy {
    in_config: InBoundConfig,
    out_config: OutBoundConfig,

    listener_config: ListenerConfig,
}

impl TcpProxy {
    pub fn build_with_config(config: &ProxyConfig) -> Self {
        Self {
            in_config: config.inbound.clone(),
            out_config: config.outbound.clone(),
            listener_config: config.listener.clone(),
        }
    }

    pub async fn io_loop(&mut self) -> Result<()> {
        // bind inbound port
        let local_addr = self.in_config.addr.clone();
        let peer_addr = self.out_config.addr.clone();
        let listener_config = self.listener_config.clone();
        let listener = TcpListener::bind_with_config(local_addr, &listener_config)
            .expect(&format!("cannot bind with address: {}", local_addr));
        // start io loop
        let io = monoio::spawn(async move {
            loop {
                let (mut conn, _addr) = listener.accept().await.expect("accept failed");
                println!("peer addr is {:?}", peer_addr);
                let mut remote_conn = TcpStream::connect_addr(peer_addr)
                    .await
                    .expect(&format!("unable to connect addr: {}", peer_addr));

                let (mut local_read, mut local_write) = conn.split();
                let (mut remote_read, mut remote_write) = remote_conn.split();
                let _ = monoio::join!(
                    copy_data(&mut local_read, &mut remote_write),
                    copy_data(&mut remote_read, &mut local_write)
                );
                println!("transfer finished");
            }
        });
        io.await;
        Ok(())
    }
}
