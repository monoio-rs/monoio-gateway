use monoio_gateway::{
    init_env,
    proxy::{tcp::TcpProxy, Proxy},
};
use monoio_gateway_core::{
    dns::tcp::TcpAddress,
    http::router::{RouterConfig, RouterRule},
};

/// a simple tcp proxy
#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), anyhow::Error> {
    init_env();
    let target = TcpAddress::new("127.0.0.1:8000".parse().expect("tcp address is not valid"));
    let _listen_port = 5000;
    let server_name = "".to_string();
    let router_config = vec![RouterConfig {
        server_name: server_name.to_owned(),
        listen_port: vec![80],
        rules: vec![RouterRule {
            path: "".to_string(),
            proxy_pass: target.clone(),
        }],
        tls: None,
    }];
    let tcp_proxy = TcpProxy::build_with_config(&router_config);
    tcp_proxy.io_loop().await?;
    Ok(())
}
