use std::{
    future::Future,
    net::{SocketAddr, ToSocketAddrs},
};

use anyhow::anyhow;
use http::Uri;

use super::Resolvable;

#[derive(Clone)]
pub struct Domain {
    uri: Uri,
}

impl Resolvable for Domain {
    type Error = anyhow::Error;

    type Item<'a> = SocketAddr
    where
        Self: 'a;

    type ResolveFuture<'a> = impl Future<Output = Result<Option<Self::Item<'a>>, Self::Error>>
    where
        Self: 'a;

    fn resolve(&self) -> Self::ResolveFuture<'_> {
        async {
            let host = self.uri.host();
            if let Some(host) = host {
                match host.to_socket_addrs() {
                    Ok(mut addrs) => Ok(addrs.next()),
                    Err(_e) => Err(anyhow!("error resolve domain {}.", host)),
                }
            } else {
                Ok(None)
            }
        }
    }
}
