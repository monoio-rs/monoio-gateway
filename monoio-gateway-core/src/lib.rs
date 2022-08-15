#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

pub mod config;
pub mod dns;
pub mod error;
pub mod http;
pub mod service;
pub mod transfer;
pub mod util;

pub(crate) trait Builder<Config> {
    fn build_with_config(config: Config) -> Self;
}
