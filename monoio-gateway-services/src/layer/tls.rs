use std::{future::Future, net::SocketAddr};

use anyhow::bail;
use monoio::net::TcpStream;
use monoio_gateway_core::{
    error::GError,
    service::{Layer, Service},
};
use monoio_rustls::TlsAcceptor;
use rustls::{Certificate, PrivateKey, ServerConfig};

use super::accept::TcpAccept;

pub type CertItem = (Vec<Certificate>, PrivateKey);

pub type TlsAccept = (monoio_rustls::ServerTlsStream<TcpStream>, SocketAddr);

#[derive(Clone)]
pub struct TlsService<T> {
    // enable_client_auth: bool,
    // cert
    config: ServerConfig,
    inner: T,
}

/// Reserved TLS trait
pub trait Tls {
    type Response<'cx>: Future<Output = Result<&'cx CertItem, GError>>
    where
        Self: 'cx;

    fn get_server_certs(&self) -> Self::Response<'_>;
}

impl<T> Service<TcpAccept> for TlsService<T>
where
    T: Service<TlsAccept>,
{
    type Response = T::Response;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, accept: TcpAccept) -> Self::Future<'_> {
        let tls_config = self.config.clone();
        async move {
            let tls_acceptor = TlsAcceptor::from(tls_config);
            match tls_acceptor.accept(accept.0).await {
                Ok(stream) => match self.inner.call((stream, accept.1)).await {
                    Ok(resp) => Ok(resp),
                    Err(err) => {
                        bail!("{}", err)
                    }
                },
                Err(err) => bail!("tls error: {}", err),
            }
        }
    }
}

#[derive(Clone)]
pub struct TlsLayer {
    enable_client_auth: bool,
    // cert
    config: ServerConfig,
}

impl TlsLayer {
    pub fn new_with_cert(
        ca_cert: String,
        crt_cert: String,
        private_key: String,
    ) -> Result<Self, GError> {
        let ca = std::fs::read(ca_cert)?;
        let ca_cert = Certificate(ca);
        let crt = std::fs::read(crt_cert)?;
        let crt_cert = Certificate(crt);
        let private = std::fs::read(private_key)?;
        let private_cert = PrivateKey(private);

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![crt_cert, ca_cert], private_cert)
            .expect("invalid server ssl cert. Please check validity of cert provided.");
        Ok(Self {
            config,
            enable_client_auth: false,
        })
    }

    pub fn enable_client_auth(mut self, enable: bool) -> Self {
        self.enable_client_auth = enable;
        self
    }
}

impl<S> Layer<S> for TlsLayer {
    type Service = TlsService<S>;

    fn layer(&self, service: S) -> Self::Service {
        TlsService {
            // enable_client_auth: self.enable_client_auth,
            config: self.config.clone(),
            inner: service,
        }
    }
}
