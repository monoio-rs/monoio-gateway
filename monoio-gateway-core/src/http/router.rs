use std::{collections::HashMap, path::Path};

use anyhow::bail;
use log::info;
use monoio_http::ParamRef;
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};

use crate::{dns::Resolvable, error::GError, Builder, MAX_CONFIG_SIZE_LIMIT};

type RouterMap<A> = HashMap<u16, Vec<RouterConfig<A>>>;

#[derive(Clone, Serialize, Deserialize)]
pub struct RoutersConfig<A> {
    pub configs: Vec<RouterConfig<A>>,
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

impl<A> Builder<RoutersConfig<A>> for Router<A>
where
    A: Resolvable,
{
    fn build_with_config(config: RoutersConfig<A>) -> Self {
        let mut rule_map = RouterMap::new();
        for conf in config.configs {
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
    pub async fn read_from_file(path: impl AsRef<Path>) -> Result<RoutersConfig<A>, GError> {
        match monoio::fs::File::open(path).await {
            Ok(f) => {
                let buf = vec![0; MAX_CONFIG_SIZE_LIMIT];
                let (sz, buf) = f.read_at(buf, 0).await;
                let len = sz?;
                info!("read {} bytes from config", len);
                let raw = &buf[..len];
                let router_config = serde_json::from_slice::<RoutersConfig<A>>(raw)?;
                Ok(router_config)
            }
            Err(err) => bail!("Error open file: {}", err),
        }
    }
}
