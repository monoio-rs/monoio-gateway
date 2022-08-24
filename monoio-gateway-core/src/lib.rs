#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

#[cfg(feature = "acme")]
pub mod acme;
pub mod balance;
pub mod config;
pub mod discover;
pub mod dns;
pub mod error;
pub mod http;
pub mod service;
pub mod transfer;
pub mod util;

use figlet_rs::FIGfont;

#[cfg(feature = "acme")]
use lazy_static::lazy_static;

const MAX_CONFIG_SIZE_LIMIT: usize = 8072;

#[cfg(feature = "acme")]
lazy_static! {
    /// editable acme dir
    pub static ref ACME_DIR: String = String::from("/usr/local/monoio-gateway/acme");
}

pub trait Builder<Config> {
    fn build_with_config(config: Config) -> Self;
}

pub fn print_logo() {
    let standard_font = FIGfont::standand().unwrap();
    if let Some(figure) = standard_font.convert("Monoio Gateway") {
        println!("{}", figure);
    }
}
