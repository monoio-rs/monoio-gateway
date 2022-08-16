#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

pub mod config;
pub mod dns;
pub mod error;
pub mod http;
pub mod service;
pub mod transfer;
pub mod util;

use figlet_rs::FIGfont;

const MAX_CONFIG_SIZE_LIMIT: usize = 8072;

pub trait Builder<Config> {
    fn build_with_config(config: Config) -> Self;
}

pub fn print_logo() {
    let standard_font = FIGfont::standand().unwrap();
    if let Some(figure) = standard_font.convert("Monoio Gateway") {
        println!("{}", figure);
    }
}
