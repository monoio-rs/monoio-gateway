use std::future::Future;

use anyhow::bail;
use log::info;
use monoio::net::{ListenerConfig, TcpListener};
use monoio_gateway_core::{
    error::GError,
    service::{Layer, Service},
};
#[derive(Clone)]
pub struct TcpListenService<T> {
    inner: T,
    listen_port: u16,
    allow_lan: bool,
    listener_config: ListenerConfig,
}

impl<T> Service<()> for TcpListenService<T>
where
    T: Service<TcpListener>,
{
    type Response = T::Response;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, _: ()) -> Self::Future<'_> {
        async {
            info!("binding port: {}", self.listen_port);
            let listen_addr = if self.allow_lan {
                format!("0.0.0.0:{}", self.listen_port)
            } else {
                format!("127.0.0.1:{}", self.listen_port)
            };
            let listener = TcpListener::bind_with_config(listen_addr, &self.listener_config)
                .expect("err bind address");
            // call listener
            match self.inner.call(listener).await {
                Ok(resp) => Ok(resp),
                Err(e) => bail!("{}", e),
            }
        }
    }
}

#[derive(Default)]
pub struct TcpListenLayer {
    listen_port: u16,
    allow_lan: bool,
    listener_config: ListenerConfig,
}

impl TcpListenLayer {
    pub fn new(listen_port: u16, allow_lan: bool) -> Self {
        TcpListenLayer {
            listen_port,
            allow_lan,
            ..Default::default()
        }
    }

    pub fn new_allow_lan(listen_port: u16) -> Self {
        TcpListenLayer {
            listen_port,
            allow_lan: true,
            ..Default::default()
        }
    }
}

impl<S> Layer<S> for TcpListenLayer {
    type Service = TcpListenService<S>;

    fn layer(&self, service: S) -> Self::Service {
        TcpListenService {
            inner: service,
            allow_lan: self.allow_lan,
            listener_config: self.listener_config.clone(),
            listen_port: self.listen_port,
        }
    }
}
