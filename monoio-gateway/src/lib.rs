#![feature(type_alias_impl_trait)]

pub mod gateway;
pub mod proxy;

pub trait ParamRef<T> {
    fn param_ref(&self) -> &T;
}

pub trait ParamMut<T> {
    fn param_mut(&mut self) -> &mut T;
}

pub fn init_env() {
    env_logger::init();
}
