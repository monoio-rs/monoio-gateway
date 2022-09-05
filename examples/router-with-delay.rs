use monoio_gateway::{
    gateway::{Gateway, Gatewayable, Servable},
    init_env,
};
use monoio_gateway_core::{
    dns::http::Domain,
    error::GError,
    http::router::{Router, RouterConfig, RouterRule, RoutersConfig},
    Builder,
};

/// This is an example to builder to proxy with 1s delay per request
#[monoio::main(timer_enabled = true)]
pub async fn main() -> Result<(), GError> {
    init_env();
    let domain = Domain::with_uri("http://127.0.0.1:8000".parse()?);
    let server_name = "python.server:5000".to_string();
    let listen_port = 5000;
    let router_config = RouterConfig {
        server_name: server_name.clone(),
        listen_port: vec![listen_port],
        rules: vec![RouterRule {
            path: "/".to_string(),
            proxy_pass: domain.clone(),
        }],
        tls: None,
    };
    let conf = RoutersConfig {
        configs: vec![router_config],
    };
    let router = Router::build_with_config(conf);
    let gws = Gateway::from_router(router);
    let _ = gws.serve().await;
    Ok(())
}
