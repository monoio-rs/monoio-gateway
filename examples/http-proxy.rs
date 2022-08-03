use std::marker::PhantomData;

use monoio::net::ListenerConfig;
use monoio_gateway::{
    config::{Config, HttpInBoundConfig, HttpOutBoundConfig, ServerConfig},
    dns::http::Domain,
    gateway::GatewayAgentable,
    proxy::h1::HttpProxyConfig,
};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), anyhow::Error> {
    let inbound_addr = Domain::new("http", "python.server:2000", "");
    let outbound_addr = Domain::new("http", "127.0.0.1:8000", "");

    let config: Config<Domain> = Config::new().push(HttpProxyConfig {
        inbound: HttpInBoundConfig::new(ServerConfig::new(inbound_addr)),
        outbound: HttpOutBoundConfig::new(ServerConfig::new(outbound_addr)),
        listener: ListenerConfig::default(),
        phantom_data: PhantomData,
    });
    let mut agent = config.build();
    agent.serve().await?;
    Ok(())
}
