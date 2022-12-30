use std::{fmt::Debug, fs::File, io::BufReader, path::Path};

use anyhow::bail;
use monoio_rustls::TlsConnector;
use rustls::server::ResolvesServerCert;

use crate::{error::GError, CERTIFICATE_MAP, DEFAULT_SSL_CLIENT_CONFIG};

pub struct CertificateResolver {
    server_name: String,
}

impl CertificateResolver {
    pub fn new(server_name: String) -> Self {
        CertificateResolver { server_name }
    }
}

/// Certificate of Monoio Gateway
/// (pem_file, private key)
pub type GatewayCertificate = (Vec<u8>, Vec<u8>);

impl ResolvesServerCert for CertificateResolver {
    fn resolve(
        &self,
        _client_hello: rustls::server::ClientHello,
    ) -> Option<std::sync::Arc<rustls::sign::CertifiedKey>> {
        let map = CERTIFICATE_MAP.read().unwrap();
        let server_name = self.server_name.as_str();
        let item = map.get(server_name);
        log::info!("certificate lookup succeed: {}", item.is_some());
        match item {
            Some(item) => Some(item.to_owned()),
            None => None,
        }
    }
}

pub fn read_pem_chain_file(path: impl AsRef<Path> + Debug + Clone) -> Result<Vec<Vec<u8>>, GError> {
    let f = File::open(path.clone())?;
    let mut reader = BufReader::new(f);
    let pems = rustls_pemfile::certs(&mut reader)?;
    Ok(pems)
}

pub fn read_pem_chain<R>(read: R) -> Result<Vec<Vec<u8>>, GError>
where
    R: std::io::Read,
{
    let mut reader = BufReader::new(read);
    let pems = rustls_pemfile::certs(&mut reader)?;
    log::info!("read pem chain length: {}", pems.len());
    Ok(pems)
}

/// read only one pem
pub fn read_pem_file(path: impl AsRef<Path> + Debug + Clone) -> Result<Vec<u8>, GError> {
    let f = File::open(path.clone())?;
    read_pem_certificate(f)
}

pub fn read_pem_certificate<R>(read: R) -> Result<Vec<u8>, GError>
where
    R: std::io::Read,
{
    let mut reader = BufReader::new(read);
    let mut pems = rustls_pemfile::certs(&mut reader)?;
    match pems.pop() {
        Some(pem) => Ok(pem),
        None => bail!("pem file validate failed"),
    }
}

pub fn read_private_key_file(path: impl AsRef<Path> + Debug + Clone) -> Result<Vec<u8>, GError> {
    let f = File::open(path.clone())?;
    read_private_key(f)
}

pub fn read_private_key<R>(read: R) -> Result<Vec<u8>, GError>
where
    R: std::io::Read,
{
    let mut reader = BufReader::new(read);
    let mut pems = rustls_pemfile::pkcs8_private_keys(&mut reader)?;
    if pems.is_empty() {
        bail!("no private key read");
    }
    match pems.pop() {
        Some(pem) => Ok(pem),
        None => bail!("private key validate failed"),
    }
}

#[inline]
pub fn get_default_tls_connector() -> TlsConnector {
    TlsConnector::from(DEFAULT_SSL_CLIENT_CONFIG.clone())
}
