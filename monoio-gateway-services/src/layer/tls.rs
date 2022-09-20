use std::{future::Future, marker::PhantomData, net::SocketAddr};

use anyhow::bail;
use log::info;
use monoio::io::{AsyncReadRent, AsyncWriteRent, Split};
use monoio_gateway_core::{
    error::GError,
    http::ssl::{read_pem_file, read_private_key_file},
    service::{Layer, Service},
    CERTIFICATE_RESOLVER,
};
use monoio_rustls::{ServerTlsStream, TlsAcceptor};
use rustls::{Certificate, PrivateKey, ServerConfig};

use super::accept::Accept;

pub type CertItem = (Vec<Certificate>, PrivateKey);
pub type TlsAccept<S> = (ServerTlsStream<S>, SocketAddr, PhantomData<S>);

#[derive(Clone)]
pub struct TlsService<T> {
    // enable_client_auth: bool,
    // cert
    config: Option<ServerConfig>,
    inner: T,
}

/// Reserved TLS trait
pub trait Tls {
    type Response<'cx>: Future<Output = Result<&'cx CertItem, GError>>
    where
        Self: 'cx;

    fn get_server_certs(&self) -> Self::Response<'_>;
}

impl<T, S> Service<Accept<S>> for TlsService<T>
where
    T: Service<TlsAccept<S>>,
    S: Split + AsyncReadRent + AsyncWriteRent,
{
    type Response = T::Response;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, accept: Accept<S>) -> Self::Future<'_> {
        let tls_config = self.config.clone();
        async move {
            info!("begin handshake");
            let tls_acceptor: TlsAcceptor;
            match tls_config {
                Some(tls_config) => {
                    tls_acceptor = TlsAcceptor::from(tls_config);
                }
                None => {
                    // default acme cert
                    let config = ServerConfig::builder()
                        .with_safe_defaults()
                        .with_no_client_auth()
                        .with_cert_resolver(CERTIFICATE_RESOLVER.clone());

                    tls_acceptor = TlsAcceptor::from(config);
                }
            }
            match tls_acceptor.accept(accept.0).await {
                Ok(stream) => match self.inner.call((stream, accept.1, PhantomData)).await {
                    Ok(resp) => Ok(resp),
                    Err(err) => {
                        bail!("{}", err)
                    }
                },
                Err(err) => bail!("tls error: {:?}", err),
            }
        }
    }
}

#[derive(Clone)]
pub struct TlsLayer {
    enable_client_auth: bool,
    // cert
    config: Option<ServerConfig>,
}

impl TlsLayer {
    pub fn new_with_cert(
        ca_cert: String,
        crt_cert: String,
        private_key: String,
    ) -> Result<Self, GError> {
        let ca = read_pem_file(ca_cert)?;
        let ca_cert = Certificate(ca);
        let crt = read_pem_file(crt_cert)?;
        let crt_cert = Certificate(crt);
        let private = read_private_key_file(private_key)?;
        let private_cert = PrivateKey(private);

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![crt_cert, ca_cert], private_cert)
            .expect("invalid server ssl cert. Please check validity of cert provided.");
        Ok(Self {
            config: Some(config),
            enable_client_auth: false,
        })
    }

    pub fn enable_client_auth(mut self, enable: bool) -> Self {
        self.enable_client_auth = enable;
        self
    }

    pub fn new() -> Self {
        Self {
            config: None,
            enable_client_auth: false,
        }
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
