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
    pub server_name: String,
    pub listen_port: u16,
    pub rules: Vec<RouterRule<A>>,
    pub tls: Option<TlsConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub mail: String,
}

impl<A> RouterConfig<A> {
    pub fn get_rules(&self) -> &Vec<RouterRule<A>> {
        &self.rules
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RouterRule<A> {
    pub path: String,
    pub proxy_pass: A,
    // TODO
}

impl<A> RouterRule<A> {
    pub fn get_path(&self) -> &String {
        &self.path
    }

    pub fn get_proxy_pass(&self) -> &A {
        &self.proxy_pass
    }
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
            info!("building {}", conf.server_name);
            if !rule_map.contains_key(&conf.listen_port) {
                rule_map.insert(conf.listen_port, vec![]);
            }
            rule_map
                .entry(conf.listen_port)
                .and_modify(|conf_vec| conf_vec.push(conf.clone()));
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
                info!("gateway count: {}", router_config.configs.len());
                Ok(router_config)
            }
            Err(err) => bail!("Error open file: {}", err),
        }
    }
}
