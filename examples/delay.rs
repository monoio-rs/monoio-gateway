use std::time::Duration;

use monoio::net::ListenerConfig;
use monoio_gateway::{
    gateway::{GatewayAgentable, HttpInBoundConfig, HttpOutBoundConfig},
    proxy::h1::HttpProxyConfig,
};
use monoio_gateway_core::{config::ServerConfig, dns::http::Domain, service::ServiceBuilder};
use monoio_gateway_services::layer::{
    accept::TcpAcceptLayer, delay::DelayLayer, listen::TcpListenLayer,
};

#[monoio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let inbound_addr = Domain::new("http", "python.server:2000", "/");
    let outbound_addr = Domain::new("http", "127.0.0.1:8000", "/");

    let proxy_config = HttpProxyConfig {
        inbound: HttpInBoundConfig::new(ServerConfig::new(inbound_addr)),
        outbound: HttpOutBoundConfig::new(ServerConfig::new(outbound_addr)),
        listener: ListenerConfig::default(),
    };

    // TODO
    let _svc_builder = ServiceBuilder::default()
        .layer(TcpListenLayer::new(proxy_config))
        .layer(TcpAcceptLayer::default())
        .layer(DelayLayer::new(Duration::from_secs(1)));
    // .service(TransferService)

    Ok(())
}
