use monoio::net::ListenerConfig;
use monoio_gateway::{
    gateway::{GatewayAgent, GatewayAgentable, HttpInBoundConfig, HttpOutBoundConfig},
    proxy::h1::HttpProxyConfig,
};
use monoio_gateway_core::{
    config::{Config, ServerConfig},
    dns::http::Domain,
};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), anyhow::Error> {
    let inbound_addr = Domain::new("http", "python.server:2000", "/");
    let outbound_addr = Domain::new("http", "127.0.0.1:8000", "/");

    let config: Config<Domain> = Config::new().push(HttpProxyConfig {
        inbound: HttpInBoundConfig::new(ServerConfig::new(inbound_addr)),
        outbound: HttpOutBoundConfig::new(ServerConfig::new(outbound_addr)),
        listener: ListenerConfig::default(),
    });
    let mut agent = GatewayAgent::<Domain>::build(&config);
    match agent.serve().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    Ok(())
}
