use std::{collections::HashMap, path::Path};

use anyhow::bail;
use monoio_http::ParamRef;
use serde::{de::DeserializeOwned};
use serde_derive::{Deserialize, Serialize};

use crate::{dns::Resolvable, error::GError, Builder};

type RouterMap<A> = HashMap<u16, Vec<RouterConfig<A>>>;

#[derive(Clone, Serialize, Deserialize)]
pub struct RoutersConfig<A> {
    configs: RouterConfig<A>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RouterConfig<A> {
    server_name: String,
    listen_port: u16,
    rules: Vec<RouterRule<A>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RouterRule<A> {
    path: String,
    proxy_pass: A,
    // TODO
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Router<A> {
    map: RouterMap<A>,
}

impl<A> Builder<Vec<RouterConfig<A>>> for Router<A>
where
    A: Resolvable,
{
    fn build_with_config(config: Vec<RouterConfig<A>>) -> Self {
        let mut rule_map = RouterMap::new();
        for conf in config.into_iter() {
            rule_map
                .entry(conf.listen_port)
                .and_modify(|conf_vec| conf_vec.push(conf.clone()))
                .or_insert(vec![]);
        }
        Self { map: rule_map }
    }
}

impl<A> ParamRef<RouterMap<A>> for Router<A> {
    fn param_ref(&self) -> &RouterMap<A> {
        &self.map
    }
}

impl<'cx, A> RouterConfig<A>
where
    A: Resolvable + DeserializeOwned,
{
    pub fn read_from_file(path: impl AsRef<Path>) -> Result<RoutersConfig<A>, GError> {
        match std::fs::read(path) {
            Ok(raw) => {
                let routers: Result<RoutersConfig<A>, serde_json::Error> =
                    serde_json::from_slice(&raw);
                match routers {
                    Ok(configs) => Ok(configs),
                    Err(err) => bail!("{}", err),
                }
            }
            Err(err) => {
                bail!("{}", err)
            }
        }
    }
}
