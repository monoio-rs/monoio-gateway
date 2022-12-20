#![feature(type_alias_impl_trait)]

use std::thread;

use anyhow::{bail, Result};
use clap::Parser;

use log::info;
use monoio::RuntimeBuilder;
use monoio_gateway::{
    gateway::{Gateway, Gatewayable, Servable},
    init_env,
};
use monoio_gateway_core::{
    dns::{http::Domain, Resolvable},
    error::GError,
    http::router::{Router, RouterConfig, RoutersConfig},
    max_parallel_count, print_logo, Builder, MAX_IOURING_ENTRIES,
};

use serde::de::DeserializeOwned;

pub mod gateway;
pub mod proxy;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path of the config file
    #[clap(short, long, value_parser)]
    config: String,
}

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<()> {
    print_logo();
    init_env();
    let args = Args::parse();
    // read config from file
    let configs = load_runtime::<Domain>(&args).await?;
    // build runtime
    let router = Router::build_with_config(configs);
    // start service
    let gws = Gateway::from_router(router);
    serve_gateway(gws);
    Ok(())
}

async fn load_runtime<A>(config: &Args) -> Result<RoutersConfig<A>, GError>
where
    A: Resolvable + DeserializeOwned,
{
    let path = config.config.to_owned();
    match RouterConfig::<A>::read_from_file(path).await {
        Ok(confs) => Ok(confs),
        Err(err) => {
            log::error!("{}", err);
            bail!("{}", err);
        }
    }
}

/// Serve Monoio-Gateway with maximum parallel count
fn serve_gateway<A>(gws: Vec<Gateway<A>>)
where
    A: Resolvable + Send + 'static,
    Gateway<A>: Gatewayable<A>,
{
    let mut handlers = vec![];
    let parallel_cnt = max_parallel_count().get();
    info!(
        "🚀 boost monoio-gateway with maximum {} worker(s).",
        parallel_cnt
    );
    for _ in 0..parallel_cnt {
        let local_gws = gws.clone();
        let handler = thread::spawn(move || {
            let mut rt = RuntimeBuilder::<monoio::IoUringDriver>::new()
                .enable_timer()
                .with_entries(MAX_IOURING_ENTRIES)
                .build()
                .unwrap();
            rt.block_on(async move {
                match local_gws.serve().await {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Gateway Error: {}", err);
                    }
                }
            });
        });
        handlers.push(handler);
    }
    for handler in handlers.into_iter() {
        let _ = handler.join();
    }
}
