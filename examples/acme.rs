use monoio_gateway::{
    gateway::{GatewayAgent, GatewayAgentable},
    init_env,
};
use monoio_gateway_core::{
    dns::http::Domain,
    error::GError,
    http::router::{RouterConfig, RouterRule, TlsConfig},
};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), GError> {
    init_env();
    let server_name = "monoio-gateway.kingtous.cn";
    let mail = "me@kingtous.cn";

    // build gateway
    let server_config = RouterConfig {
        server_name: server_name.to_string(),
        listen_port: 80,
        rules: vec![RouterRule {
            path: "/".to_string(),
            proxy_pass: Domain::with_uri("https://cv.kingtous.cn".parse().unwrap()),
        }],
        tls: Some(TlsConfig {
            mail: mail.to_string(),
            // None to use
            root_ca: None,
            server_key: None,
            private_key: None,
            acme_uri: None,
        }),
    };
    let mut gateway = GatewayAgent::<Domain>::build(&vec![server_config]);
    let _ = gateway.serve().await;
    Ok(())
}
