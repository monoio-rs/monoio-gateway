use std::{time::Duration, marker::PhantomData, any};

use monoio::{net::ListenerConfig, time::Timeout};
use monoio_gateway::{dns::http::Domain, config::{Config, HttpInBoundConfig, ServerConfig, HttpOutBoundConfig}, proxy::h1::HttpProxyConfig, gateway::GatewayAgentable};
use monoio_gateway_core::service::ServiceBuilder;
use monoio_gateway_services::layer::{delay::DelayLayer, timeout::{TimeoutLayer, TimeoutService}};

#[monoio::main]
pub async fn main() -> Result<(), anyhow::Error>{
    let delay = DelayLayer::new(Duration::from_secs(1));
    let timeout = TimeoutLayer::new(Duration::from_secs(1));

    let svc_builder = ServiceBuilder::default();
    let _svc = svc_builder.layer(delay);

    let inbound_addr = Domain::new("http", "python.server:2000", "/");
    let outbound_addr = Domain::new("http", "127.0.0.1:8000", "/");

    let config: Config<Domain> = Config::new().push(HttpProxyConfig {
        inbound: HttpInBoundConfig::new(ServerConfig::new(inbound_addr)),
        outbound: HttpOutBoundConfig::new(ServerConfig::new(outbound_addr)),
        listener: ListenerConfig::default(),
        phantom_data: PhantomData,
    });
    let mut agent = config.build();
    match agent.serve().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    Ok(())
}
