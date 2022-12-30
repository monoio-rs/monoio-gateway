use std::{collections::HashMap, ffi::OsStr, os::unix::prelude::OsStrExt, path::Path};

use anyhow::bail;
use log::{error, info};
use monoio_http::ParamRef;
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};

use crate::{dns::Resolvable, error::GError, Builder, MAX_CONFIG_SIZE_LIMIT};

use super::version::Type;

type RouterMap<A> = HashMap<u16, RouterConfig<A>>;

#[derive(Clone, Serialize, Deserialize)]
pub struct RoutersConfig<A> {
    pub configs: Vec<RouterConfig<A>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RouterConfig<A> {
    pub server_name: String,
    #[serde(default)]
    pub protocol: Type,
    pub listen_port: Vec<u16>,
    pub rules: Vec<RouterRule<A>>,
    pub tls: Option<TlsConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub mail: String,
    pub chain: Option<String>,
    pub private_key: Option<String>,
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
            for listen_port in conf.listen_port.iter() {
                if rule_map.contains_key(listen_port) {
                    error!(
                        "server {} listen port {} duplicate, ignore",
                        conf.server_name, listen_port
                    );
                    continue;
                }
                rule_map.insert(*listen_port, conf.clone());
            }
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
        let extension = path
            .as_ref()
            .extension()
            .unwrap_or(OsStr::from_bytes("json".as_bytes()))
            .to_ascii_lowercase();
        match monoio::fs::File::open(path).await {
            Ok(f) => {
                let buf = vec![0; MAX_CONFIG_SIZE_LIMIT];
                let (sz, buf) = f.read_at(buf, 0).await;
                let len = sz?;
                info!(
                    "read {} bytes from config with extension {:?}",
                    len, extension
                );
                let raw = &buf[..len];
                match extension.to_str() {
                    Some("json") => {
                        let router_config = serde_json::from_slice::<RoutersConfig<A>>(raw)?;
                        info!("gateway count: {}", router_config.configs.len());
                        Ok(router_config)
                    }
                    Some("toml") => {
                        let router_config = toml::from_slice::<RoutersConfig<A>>(raw)?;
                        info!("gateway count: {}", router_config.configs.len());
                        Ok(router_config)
                    }
                    _ => bail!("Unsupport file type: {:?}", extension),
                }
            }
            Err(err) => bail!("Error open file: {}", err),
        }
    }
}
