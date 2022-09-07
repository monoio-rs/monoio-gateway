use monoio_gateway::{
    gateway::{Gateway, Gatewayable, Servable},
    init_env,
};
use monoio_gateway_core::{
    dns::http::Domain,
    http::router::{Router, RouterConfig, RouterRule, RoutersConfig, TlsConfig},
    Builder,
};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), anyhow::Error> {
    init_env();
    let domain = Domain::with_uri("http://127.0.0.1:8000".parse()?);
    let server_name = "monoio-gateway.kingtous.cn".to_string();
    let router_config = RouterConfig {
        server_name: server_name.clone(),
        listen_port: vec![80, 443],
        rules: vec![RouterRule {
            path: "/".to_string(),
            proxy_pass: domain.clone(),
        }],
        tls: Some(TlsConfig {
            mail: "me@kingtous.cn".into(),
            chain: None,
            private_key: None,
        }),
    };
    let conf = RoutersConfig {
        configs: vec![router_config],
    };
    let router = Router::build_with_config(conf);
    let gws = Gateway::from_router(router);
    let _ = gws.serve().await;
    Ok(())
}
