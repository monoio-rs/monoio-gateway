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

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use figlet_rs::FIGfont;

#[cfg(feature = "acme")]
use lazy_static::lazy_static;
use rustls::{OwnedTrustAnchor, RootCertStore};

use crate::{dns::http::Domain, http::ssl::CertificateResolver};

pub const MAX_CONFIG_SIZE_LIMIT: usize = 8072;
pub const ACME_URI_PREFIX: &str = "/.well-known";

#[cfg(feature = "acme")]
lazy_static! {
    /// editable acme dir
    pub static ref ACME_DIR: String = String::from("/var/monoio-gateway/acme");
    /// ssl
    pub static ref CERTIFICATE_MAP: Arc<RwLock<HashMap<String, Arc<rustls::sign::CertifiedKey>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref CERTIFICATE_RESOLVER: Arc<CertificateResolver> = Arc::new(CertificateResolver::new());
    pub static ref DEFAULT_SSL_CLIENT_CONFIG: Arc<rustls::ClientConfig> = {
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
        Arc::new(config)
    };
}

lazy_static! {
    /// Service Discover
    ///
    /// Note:
    /// A thread-shared map <Server Domain, Proxy Pass Domain>
    pub static ref DISCOVERED: Arc<RwLock<HashMap<Domain, Domain>>> = Arc::new(RwLock::new(HashMap::new()));
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
