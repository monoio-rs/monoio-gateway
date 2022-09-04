use std::{fmt::Debug, fs::File, io::BufReader, path::Path};

use anyhow::bail;
use monoio_rustls::TlsConnector;
use rustls::{
    internal::msgs::codec::Codec, server::ResolvesServerCert, OwnedTrustAnchor, RootCertStore,
};

use crate::{error::GError, CERTIFICATE_MAP};

#[derive(Default)]
pub struct CertificateResolver;

impl CertificateResolver {
    pub fn new() -> Self {
        CertificateResolver::default()
    }
}

/// Certificate of Monoio Gateway
/// (pem_file, private key)
pub type GatewayCertificate = (Vec<u8>, Vec<u8>);

impl ResolvesServerCert for CertificateResolver {
    fn resolve(
        &self,
        client_hello: rustls::server::ClientHello,
    ) -> Option<std::sync::Arc<rustls::sign::CertifiedKey>> {
        match client_hello.server_name() {
            Some(server_name) => {
                let map = CERTIFICATE_MAP.read().unwrap();
                let item = map.get(server_name);
                match item {
                    Some(item) => Some(item.to_owned()),
                    None => None,
                }
            }
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
    let mut pems = rustls_pemfile::rsa_private_keys(&mut reader)?;
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
    let mut root_store = RootCertStore::empty();
    root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    TlsConnector::from(config)
}
