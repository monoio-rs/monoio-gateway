#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use anyhow::{bail, Result};
use clap::Parser;
use monoio_gateway::{
    gateway::{GatewayAgent, GatewayAgentable},
    init_env,
};
use monoio_gateway_core::{
    dns::{http::Domain, Resolvable},
    error::GError,
    http::router::{Router, RouterConfig, RoutersConfig},
    print_logo, Builder,
};
use monoio_http::ParamRef;
use serde::de::DeserializeOwned;

pub mod balance;
pub mod discover;
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
    let m = router.param_ref();
    let mut handles = vec![];
    for (_, v) in m {
        let config_vec = v.clone();
        handles.push(monoio::spawn(async move {
            let mut agent = GatewayAgent::<Domain>::build(&(config_vec.to_owned()));
            agent.serve().await
        }));
    }
    for handle in handles {
        match handle.await {
            Err(err) => {
                eprintln!("{}", err);
            }
            _ => {}
        }
    }
    Ok(())
}

async fn load_runtime<A>(config: &Args) -> Result<RoutersConfig<A>, GError>
where
    A: Resolvable + DeserializeOwned,
{
    let path = config.config.to_owned();
    match RouterConfig::<A>::read_from_file(path).await {
        Ok(confs) => Ok(confs),
        Err(err) => bail!("{}", err),
    }
}
