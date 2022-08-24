use std::{env::join_paths, ffi::OsString, fmt::Display, future::Future, path::Path};

use crate::{dns::http::Domain, error::GError, ACME_DIR};

pub mod lets_encrypt;

/// ACME agent trait
pub trait Acme {
    type Response;

    type Email;

    type Error: Display;

    type Future<'cx>: Future<Output = Result<Option<Self::Response>, Self::Error>>
    where
        Self: 'cx;

    fn acme(&self, acme_request: Self::Email) -> Self::Future<'_>;
}

/// for those domain, to get acme path
pub trait Acmed {
    fn get_acme_path(&self) -> Result<OsString, GError>;
}

pub(crate) fn get_acme_path(domain: &Domain) -> Result<OsString, GError> {
    let path = join_paths([
        Path::new(&ACME_DIR.to_string()),
        Path::new(&format!("acme/{}", domain.host())),
    ])?;
    Ok(path)
}

impl Acmed for Domain {
    fn get_acme_path(&self) -> Result<OsString, GError> {
        get_acme_path(self)
    }
}
