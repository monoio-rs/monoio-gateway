use monoio::net::ListenerConfig;
use monoio_gateway::{
    gateway::{GatewayAgentable, HttpInBoundConfig, HttpOutBoundConfig},
    init_env,
    proxy::h1::HttpProxyConfig,
};
use monoio_gateway_core::{
    config::ServerConfig,
    dns::http::Domain,
    error::GError,
    service::{Service, ServiceBuilder},
};
use monoio_gateway_services::layer::{
    accept::TcpAcceptLayer, delay::DelayLayer, dial::DialRemoteLayer, listen::TcpListenLayer,
    transfer::TransferService,
};
use std::time::Duration;

/// This is an example to builder to proxy with 1s delay per request
#[monoio::main(timer_enabled = true)]
pub async fn main() -> Result<(), GError> {
    init_env();
    let inbound_addr = Domain::new("http", "127.0.0.1:2001", "/");
    let outbound_addr = Domain::new("http", "127.0.0.1:8000", "/");

    let proxy_config = HttpProxyConfig {
        inbound: HttpInBoundConfig::new(ServerConfig::new(inbound_addr.clone())),
        outbound: HttpOutBoundConfig::new(ServerConfig::new(outbound_addr.clone())),
        listener: ListenerConfig::default(),
    };

    let mut svc = ServiceBuilder::default()
        .layer(TcpListenLayer::new(proxy_config))
        .layer(TcpAcceptLayer::default())
        .layer(DelayLayer::new(Duration::from_secs(1)))
        .layer(DialRemoteLayer::new(outbound_addr))
        .service(TransferService::default());
    svc.call(()).await?;
    Ok(())
}
