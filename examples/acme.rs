use monoio_gateway::{
    gateway::{Gateway, Gatewayable, Servable},
    init_env,
};
use monoio_gateway_core::{
    dns::http::Domain,
    error::GError,
    http::router::{Router, RouterConfig, RouterRule, RoutersConfig, TlsConfig},
    Builder,
};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), GError> {
    init_env();
    let server_name = "monoio-gateway.kingtous.cn";
    let mail = "me@kingtous.cn";
    // http handler for compatiblity
    let server_config = RouterConfig {
        server_name: server_name.to_string(),
        listen_port: vec![80, 443],
        rules: vec![RouterRule {
            path: "/".to_string(),
            proxy_pass: Domain::with_uri("https://cv.kingtous.cn".parse().unwrap()),
        }],
        tls: Some(TlsConfig {
            mail: mail.to_string(),
            // None to use prebuilt acme support
            chain: None,
            private_key: None,
        }),
    };
    let conf = RoutersConfig {
        configs: vec![server_config],
    };
    let router = Router::build_with_config(conf);
    let gws = Gateway::from_router(router);
    let _ = gws.serve().await;
    Ok(())
}
