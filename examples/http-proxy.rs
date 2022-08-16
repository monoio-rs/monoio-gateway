use std::collections::HashMap;

use monoio_gateway::{gateway::GatewayAgentable, init_env};
use monoio_gateway_core::{
    dns::http::Domain,
    http::router::{RouterConfig, RouterRule},
    service::{Service, ServiceBuilder},
};
use monoio_gateway_services::layer::{
    accept::TcpAcceptLayer, listen::TcpListenLayer, router::RouterLayer,
    transfer::HttpTransferService,
};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), anyhow::Error> {
    init_env();
    let domain = Domain::with_uri("http://127.0.0.1:8000".parse()?);
    let server_name = "python.server:5000".to_string();
    let listen_port = 5000;
    let router_config = RouterConfig {
        server_name: server_name.clone(),
        listen_port,
        rules: vec![RouterRule {
            path: "/".to_string(),
            proxy_pass: domain.clone(),
        }],
    };
    let mut route_map = HashMap::new();
    route_map.insert(server_name, router_config);
    println!("{:?}", route_map.keys());

    let mut svc = ServiceBuilder::default()
        .layer(TcpListenLayer::new_allow_lan(listen_port))
        .layer(TcpAcceptLayer::default())
        .layer(RouterLayer::new(route_map))
        .service(HttpTransferService::default());
    svc.call(()).await?;
    Ok(())
}
