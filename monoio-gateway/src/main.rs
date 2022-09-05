#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use anyhow::{bail, Result};
use clap::Parser;

use monoio_gateway::{
    gateway::{Gateway, Gatewayable, Servable},
    init_env,
};
use monoio_gateway_core::{
    dns::{http::Domain, Resolvable},
    error::GError,
    http::router::{Router, RouterConfig, RoutersConfig},
    print_logo, Builder,
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
    let _ = gws.serve().await;
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
