#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

pub mod balance;
pub mod config;
pub mod discover;
pub mod dns;
pub mod gateway;
pub mod http;
pub mod layer;
pub mod proxy;
pub mod log;

pub trait ParamRef<T> {
    fn param_ref(&self) -> &T;
}

pub trait ParamMut<T> {
    fn param_mut(&mut self) -> &mut T;
}

pub fn init_env() {
    log::init_logger();
}