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
    // http handler for compatiblity
    let server_config = RouterConfig {
        server_name: server_name.to_string(),
        listen_port: 80,
        rules: vec![RouterRule {
            path: "/".to_string(),
            proxy_pass: Domain::with_uri("https://cv.kingtous.cn".parse().unwrap()),
        }],
        tls: None,
    };
    // ssl handler
    let mut server_ssl_config = server_config.clone();
    server_ssl_config.listen_port = 443;
    server_ssl_config.tls = Some(TlsConfig {
        mail: mail.to_string(),
        // None to use prebuilt acme support
        root_ca: None,
        server_key: None,
        private_key: None,
    });
    let mut http_gateway = GatewayAgent::<Domain>::build(&vec![server_config]);
    let mut ssl_gateway = GatewayAgent::<Domain>::build(&vec![server_ssl_config]);
    let _ = monoio::join!(http_gateway.serve(), ssl_gateway.serve());
    Ok(())
}
