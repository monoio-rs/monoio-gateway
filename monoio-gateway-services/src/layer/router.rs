use std::{collections::HashMap, future::Future, path::Path, sync::Arc};

use anyhow::bail;

use bytes::Bytes;
use http::response::Builder;
use log::{debug, info};
use monoio::io::{sink::SinkExt, stream::Stream, AsyncReadRent, AsyncWriteRent, Split, Splitable};
use monoio_gateway_core::{
    acme::Acmed,
    dns::{http::Domain, Resolvable},
    error::GError,
    http::router::{RouterConfig, RouterRule},
    service::{Layer, Service},
    ACME_URI_PREFIX,
};
use monoio_http::{
    common::request::Request,
    h1::{
        codec::{decoder::RequestDecoder, encoder::GenericEncoder},
        payload::{FixedPayload, Payload},
    },
};

use crate::layer::transfer::TransferParamsType;

use super::{
    accept::Accept, detect::DetectResult, endpoint::EndpointRequestParams, tls::TlsAccept,
};
#[derive(Clone)]
pub struct RouterService<T, A> {
    inner: T,
    routes: Arc<HashMap<String, RouterConfig<A>>>,
}

/// Direct use router before Accept
impl<T, S> Service<Accept<S>> for RouterService<T, Domain>
where
    T: Service<EndpointRequestParams<Domain, Domain, S>>,
    S: Split + AsyncWriteRent + AsyncReadRent,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, local_stream: Accept<S>) -> Self::Future<'_> {
        async move {
            debug!("find route for {:?}", local_stream.1);
            let (local_read, local_write) = local_stream.0.into_split();
            let mut local_decoder = RequestDecoder::new(local_read);
            match local_decoder.next().await {
                Some(Ok(req)) => {
                    let req: Request = req;
                    let host = get_host(&req);
                    match host {
                        Some(host) => {
                            let domain = Domain::with_uri(host.parse()?);
                            let target = self.match_target(&host.to_owned());
                            match target {
                                Some(target) => {
                                    let m = longest_match(req.uri().path(), target.get_rules());
                                    if let Some(rule) = m {
                                        let proxy_pass = rule.get_proxy_pass();
                                        let local_encoder = GenericEncoder::new(local_write);
                                        // connect endpoint
                                        match self
                                            .inner
                                            .call(EndpointRequestParams::new(
                                                TransferParamsType::ServerHttp(
                                                    local_encoder,
                                                    local_decoder,
                                                    domain,
                                                ),
                                                proxy_pass.clone(),
                                                Some(req),
                                            ))
                                            .await
                                        {
                                            Ok(resp) => {
                                                return Ok(Some(resp));
                                            }
                                            Err(err) => {
                                                bail!("endpoint communication failed: {}", err)
                                            }
                                        }
                                    } else {
                                        // no match router rule
                                        debug!("no matching router rule, {}", domain);
                                        if let Ok(handled) = self
                                            .handle_acme_verification(req, target, local_write)
                                            .await
                                        {
                                            if handled {
                                                return Ok(None);
                                            }
                                        }
                                        debug!("no matching router rule, {}", domain);
                                    }
                                }
                                None => {
                                    debug!("no matching endpoint, ignoring {}", domain);
                                }
                            }
                        }
                        None => {
                            // no host, ignore!
                            debug!("request has no host, uri: {}", req.uri());
                        }
                    }
                }
                Some(Err(err)) => {
                    // TODO: fallback to tcp
                    debug!("detect failed, fallback to tcp: {:?}", local_stream.1);
                    bail!("{}", err)
                }
                _ => {}
            }
            Ok(None)
        }
    }
}

/// Direct use router before Accept
impl<T, S> Service<TlsAccept<S>> for RouterService<T, Domain>
where
    T: Service<EndpointRequestParams<Domain, Domain, S>>,
    S: Split + AsyncWriteRent + AsyncReadRent,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, local_stream: TlsAccept<S>) -> Self::Future<'_> {
        async move {
            debug!("find route for {:?}", local_stream.1);
            let (local_read, local_write) = local_stream.0.split();
            let local_encoder = GenericEncoder::new(local_write);
            let mut local_decoder = RequestDecoder::new(local_read);
            match local_decoder.next().await {
                Some(Ok(req)) => {
                    let req: Request = req;
                    let host = get_host(&req);
                    match host {
                        Some(host) => {
                            let domain = Domain::with_uri(host.parse()?);
                            let target = self.match_target(&host.to_owned());
                            match target {
                                Some(target) => {
                                    let m = longest_match(req.uri().path(), target.get_rules());
                                    if let Some(rule) = m {
                                        let proxy_pass = rule.get_proxy_pass();
                                        // connect endpoint
                                        match self
                                            .inner
                                            .call(EndpointRequestParams::new(
                                                TransferParamsType::ServerTls(
                                                    local_encoder,
                                                    local_decoder,
                                                    domain,
                                                ),
                                                proxy_pass.clone(),
                                                Some(req),
                                            ))
                                            .await
                                        {
                                            Ok(resp) => {
                                                return Ok(Some(resp));
                                            }
                                            Err(err) => {
                                                bail!("endpoint communication failed: {}", err)
                                            }
                                        }
                                    } else {
                                        // no match router rule
                                        debug!("no matching router rule, {}", domain);
                                        // no need to handle acme, because it's already tls connection.
                                    }
                                }
                                None => {
                                    debug!("no matching endpoint, ignoring {}", domain);
                                }
                            }
                        }
                        None => {
                            // no host, ignore!
                            debug!("request has no host, uri: {}", req.uri());
                        }
                    }
                }
                Some(Err(err)) => {
                    // TODO: fallback to tcp
                    debug!("detect failed, fallback to tcp: {:?}", local_stream.1);
                    bail!("{}", err)
                }
                _ => {}
            }
            Ok(None)
        }
    }
}

/// Support detect result
impl<T, S> Service<DetectResult<S>> for RouterService<T, Domain>
where
    T: Service<EndpointRequestParams<Domain, Domain, S>>,
    S: Split + AsyncReadRent + AsyncWriteRent,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, local_stream: DetectResult<S>) -> Self::Future<'_> {
        async move {
            let (ty, stream, _socketaddr) = local_stream;
            debug!("find route for {:?}", ty);
            let (local_read, local_write) = stream.into_split();
            let mut local_decoder = RequestDecoder::new(local_read);
            match local_decoder.next().await {
                Some(Ok(req)) => {
                    let req: Request<Payload> = req;
                    let host = get_host(&req);
                    match host {
                        Some(host) => {
                            let domain = Domain::with_uri(host.parse()?);
                            let target = self.match_target(&host.to_owned());
                            match target {
                                Some(target) => {
                                    let m = longest_match(req.uri().path(), target.get_rules());
                                    if let Some(rule) = m {
                                        let proxy_pass = rule.get_proxy_pass();
                                        let local_encoder = GenericEncoder::new(local_write);
                                        // connect endpoint
                                        match self
                                            .inner
                                            .call(EndpointRequestParams::new(
                                                TransferParamsType::ServerHttp(
                                                    local_encoder,
                                                    local_decoder,
                                                    domain,
                                                ),
                                                proxy_pass.clone(),
                                                Some(req),
                                            ))
                                            .await
                                        {
                                            Ok(resp) => {
                                                return Ok(Some(resp));
                                            }
                                            Err(err) => {
                                                bail!("endpoint communication failed: {}", err)
                                            }
                                        }
                                    } else {
                                        // no match router rule
                                        if let Ok(handled) = self
                                            .handle_acme_verification(req, target, local_write)
                                            .await
                                        {
                                            if handled {
                                                return Ok(None);
                                            }
                                        }
                                        debug!("no matching router rule, {}", domain);
                                    }
                                }
                                None => {
                                    debug!("no matching endpoint, ignoring {}", domain);
                                }
                            }
                        }
                        None => {
                            debug!("request has no host, uri: {}", req.uri());
                        }
                    }
                }
                Some(Err(err)) => {
                    // TODO: fallback to tcp
                    debug!("detect failed, fallback to tcp");
                    bail!("{}", err)
                }
                _ => {}
            }
            Ok(None)
        }
    }
}

impl<T, A> RouterService<T, A>
where
    A: Resolvable,
{
    #[inline]
    fn match_target(&self, host: &String) -> Option<&RouterConfig<A>> {
        self.routes.get(host)
    }

    /// if not handled, return false to continue handler
    async fn handle_acme_verification<S: AsyncWriteRent>(
        &self,
        req: Request<Payload>,
        conf: &RouterConfig<A>,
        stream: S,
    ) -> Result<bool, GError> {
        let name = conf.server_name.get_acme_path()?;
        let p = Path::new(&name);
        match &conf.tls {
            Some(_) => {
                let req_path = req.uri().path().to_string();
                log::info!("acme: request path: {}", req_path);
                if req_path.starts_with(ACME_URI_PREFIX) {
                    let mut encoder = GenericEncoder::new(stream);
                    // read files
                    let abs_path = p.join(&req_path[1..]);
                    log::info!("acme: read path: {:?}", abs_path);
                    let mut file_bytes = vec![];
                    match monoio::fs::File::open(Path::new(&abs_path)).await {
                        Ok(challenge_file) => {
                            let mut pos = 0;
                            loop {
                                let buf = vec![0 as u8; 1024];
                                let (n, mut read) = challenge_file.read_at(buf, pos).await;
                                let n = n? as u64;
                                if n == 0 {
                                    // EOF, let's send our challenge now.
                                    break;
                                }
                                pos += n;
                                unsafe { read.set_len(n as usize) };
                                file_bytes.append(&mut read);
                            }
                            let bytes = Bytes::from(file_bytes);
                            let response = Builder::new()
                                .body(Payload::Fixed(FixedPayload::new(bytes)))
                                .unwrap();
                            encoder.send_and_flush(response).await?;
                            info!("acme challenge replied");
                            return Ok(true);
                        }
                        Err(e) => {
                            log::warn!("find acme file error: {}", e);
                            let data = Bytes::from_static(b"404 not found --- Monoio Gateway.");
                            let response = Builder::new()
                                .body(Payload::Fixed(FixedPayload::new(data)))
                                .unwrap();
                            encoder.send_and_flush(response).await?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }
}

pub struct RouterLayer<A> {
    routes: Arc<HashMap<String, RouterConfig<A>>>,
}

impl<A> RouterLayer<A> {
    pub fn new(routes: Arc<HashMap<String, RouterConfig<A>>>) -> Self {
        Self { routes }
    }
}

impl<S, A> Layer<S> for RouterLayer<A>
where
    A: Resolvable,
{
    type Service = RouterService<S, A>;

    fn layer(&self, service: S) -> Self::Service {
        RouterService {
            inner: service,
            routes: self.routes.clone(),
        }
    }
}

#[inline]
fn longest_match<'cx>(
    req_path: &'cx str,
    routes: &'cx Vec<RouterRule<Domain>>,
) -> Option<&'cx RouterRule<Domain>> {
    info!("request path: {}", req_path);
    // TODO: opt progress
    if req_path.starts_with(ACME_URI_PREFIX) {
        return None;
    }
    let mut target_route = None;
    let mut route_len = 0;
    for route in routes.iter() {
        let route_path = route.get_path();
        let route_path_len = route_path.len();
        if req_path.starts_with(route_path) && route_path_len > route_len {
            target_route = Some(route);
            route_len = route_path_len;
        }
    }
    target_route
}

#[inline]
fn get_host(req: &Request<Payload>) -> Option<&str> {
    match req.headers().get("host") {
        Some(host) => Some(host.to_str().unwrap_or("")),
        None => None,
    }
}
