use std::{ffi::OsString, fmt::Display, fs::create_dir_all, future::Future, path::Path};

use log::info;

use crate::{dns::http::Domain, error::GError, ACME_DIR};

mod acme;

pub type GenericAcme = acme::GenericAcme;

pub use acme::{start_acme, update_certificate};

/// ACME agent trait
pub trait Acme {
    type Response;

    type Error: Display;

    type Future<'cx>: Future<Output = Result<Option<Self::Response>, Self::Error>>
    where
        Self: 'cx;

    fn acme(&self, acme_request: ()) -> Self::Future<'_>;
}

/// for those domain, to get acme path
pub trait Acmed {
    fn get_acme_path(&self) -> Result<OsString, GError>;
}

pub(crate) fn get_acme_path(domain: &str) -> Result<OsString, GError> {
    let path = Path::new(&ACME_DIR.to_string()).join(Path::new(domain));
    info!("acme path for {}: {:?}", domain, path);
    // ensure path exists
    create_dir_all(path.to_owned())?;
    Ok(path.into())
}

impl Acmed for Domain {
    fn get_acme_path(&self) -> Result<OsString, GError> {
        get_acme_path(&self.host())
    }
}

/// for convinient convert a server name to acme path
impl Acmed for &str {
    fn get_acme_path(&self) -> Result<OsString, GError> {
        get_acme_path(self)
    }
}

impl Acmed for String {
    fn get_acme_path(&self) -> Result<OsString, GError> {
        get_acme_path(&self)
    }
}
