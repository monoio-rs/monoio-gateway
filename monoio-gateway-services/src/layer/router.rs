use std::{borrow::Borrow, collections::HashMap, future::Future, path::Path, rc::Rc, sync::RwLock};

use anyhow::bail;

use bytes::Bytes;
use http::{response::Builder, StatusCode};
use log::{debug, info};
use monoio::{
    io::{
        sink::{Sink, SinkExt},
        stream::Stream,
        AsyncReadRent, AsyncWriteRent, Split, Splitable,
    },
    net::TcpStream,
};
use monoio_gateway_core::{
    acme::Acmed,
    dns::{http::Domain, Resolvable},
    error::GError,
    http::{
        router::{RouterConfig, RouterRule},
        Rewrite,
    },
    service::Service,
    transfer::{copy_response_lock, generate_response},
    ACME_URI_PREFIX,
};
use monoio_http::{
    common::{request::Request, response::Response},
    h1::{
        codec::{decoder::RequestDecoder, encoder::GenericEncoder},
        payload::{FixedPayload, Payload},
    },
};

use crate::layer::endpoint::ConnectEndpoint;

use super::{
    accept::Accept,
    endpoint::{ClientConnectionType, EndpointRequestParams},
    tls::TlsAccept,
};

pub type SharedTcpConnectPool<I, O> = Rc<RwLock<HashMap<Domain, Rc<ClientConnectionType<I, O>>>>>;

pub struct RouterService<A, I, O: AsyncWriteRent> {
    routes: Rc<HashMap<String, RouterConfig<A>>>,

    connect_pool: SharedTcpConnectPool<I, O>,
}

impl<A, I, O> Clone for RouterService<A, I, O>
where
    O: AsyncWriteRent,
{
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            connect_pool: self.connect_pool.clone(),
        }
    }
}

/// Direct use router before Accept
impl<S> Service<Accept<S>> for RouterService<Domain, TcpStream, TcpStream>
where
    S: Split + AsyncReadRent + AsyncWriteRent + 'static,
{
    type Response = ();

    type Error = GError;

    type Future<'a> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'a;

    fn call(&mut self, local_stream: Accept<S>) -> Self::Future<'_> {
        async move {
            let (stream, _socketaddr) = local_stream;
            let (local_read, local_write) = stream.into_split();
            let mut local_decoder = RequestDecoder::new(local_read);
            let local_encoder = Rc::new(RwLock::new(GenericEncoder::new(local_write)));
            loop {
                let connect_pool = self.connect_pool.clone();
                let local_encoder_clone = local_encoder.clone();
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
                                            // parsed rule for this request and spawn task to handle endpoint connection
                                            let proxy_pass = rule.get_proxy_pass().to_owned();
                                            handle_endpoint_connection(
                                                connect_pool,
                                                &proxy_pass,
                                                local_encoder_clone,
                                                req,
                                            )
                                            .await;
                                            continue;
                                        } else {
                                            // no match router rule, is acme?
                                            if let Ok(handled) = self
                                                .handle_acme_verification(
                                                    req,
                                                    target,
                                                    local_encoder_clone.clone(),
                                                )
                                                .await
                                            {
                                                // no, is not acme, not find handler
                                                if handled {
                                                    continue;
                                                }
                                            }
                                            debug!("no matching router rule, {}", domain);
                                            let _ = local_encoder_clone
                                                .write()
                                                .unwrap()
                                                .send_and_flush(generate_response(
                                                    StatusCode::NOT_FOUND,
                                                ));
                                        }
                                    }
                                    None => {
                                        debug!("no matching endpoint, ignoring {}", domain);
                                        let _ =
                                            local_encoder_clone.write().unwrap().send_and_flush(
                                                generate_response(StatusCode::NOT_FOUND),
                                            );
                                    }
                                }
                            }
                            None => {
                                debug!("request has no host, uri: {}", req.uri());
                                let _ = local_encoder_clone
                                    .write()
                                    .unwrap()
                                    .send_and_flush(generate_response(StatusCode::FORBIDDEN));
                            }
                        };
                    }
                    Some(Err(err)) => {
                        // TODO: fallback to tcp
                        debug!("detect failed, fallback to tcp");
                        bail!("{}", err);
                    }
                    _ => {
                        break;
                    }
                }
            }
            Ok(())
        }
    }
}

/// Direct use router before Accept
///
/// TODO: less copy code
impl<S> Service<TlsAccept<S>> for RouterService<Domain, TcpStream, TcpStream>
where
    S: Split + AsyncReadRent + AsyncWriteRent + 'static,
{
    type Response = ();

    type Error = GError;

    type Future<'a> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'a;

    fn call(&mut self, local_stream: TlsAccept<S>) -> Self::Future<'_> {
        async move {
            let (stream, _socketaddr, _) = local_stream;
            let (local_read, local_write) = stream.split();
            let mut local_decoder = RequestDecoder::new(local_read);
            let local_encoder = Rc::new(RwLock::new(GenericEncoder::new(local_write)));
            loop {
                let connect_pool = self.connect_pool.clone();
                let local_encoder_clone = local_encoder.clone();
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
                                            // parsed rule for this request and spawn task to handle endpoint connection
                                            let proxy_pass = rule.get_proxy_pass().to_owned();
                                            handle_endpoint_connection(
                                                connect_pool,
                                                &proxy_pass,
                                                local_encoder_clone,
                                                req,
                                            )
                                            .await;
                                            continue;
                                        } else {
                                            // no match router rule, is acme?
                                            if let Ok(handled) = self
                                                .handle_acme_verification(
                                                    req,
                                                    target,
                                                    local_encoder_clone.clone(),
                                                )
                                                .await
                                            {
                                                // no, is not acme, not find handler
                                                if handled {
                                                    continue;
                                                }
                                            }
                                            debug!("no matching router rule, {}", domain);
                                            let _ = local_encoder_clone
                                                .write()
                                                .unwrap()
                                                .send_and_flush(generate_response(
                                                    StatusCode::NOT_FOUND,
                                                ));
                                        }
                                    }
                                    None => {
                                        debug!("no matching endpoint, ignoring {}", domain);
                                        let _ =
                                            local_encoder_clone.write().unwrap().send_and_flush(
                                                generate_response(StatusCode::NOT_FOUND),
                                            );
                                    }
                                }
                            }
                            None => {
                                debug!("request has no host, uri: {}", req.uri());
                                let _ = local_encoder_clone
                                    .write()
                                    .unwrap()
                                    .send_and_flush(generate_response(StatusCode::FORBIDDEN));
                            }
                        };
                    }
                    Some(Err(err)) => {
                        // TODO: fallback to tcp
                        debug!("detect failed, fallback to tcp");
                        bail!("{}", err);
                    }
                    _ => {
                        break;
                    }
                }
            }
            Ok(())
        }
    }
}

impl<A, I, O> RouterService<A, I, O>
where
    A: Resolvable,
    O: AsyncWriteRent,
{
    pub fn new(routes: Rc<HashMap<String, RouterConfig<A>>>) -> Self {
        Self {
            routes,
            connect_pool: Default::default(),
        }
    }

    #[inline]
    fn match_target(&self, host: &String) -> Option<&RouterConfig<A>> {
        self.routes.get(host)
    }

    /// if not handled, return false to continue handler
    async fn handle_acme_verification<IO: Sink<Response>>(
        &self,
        req: Request<Payload>,
        conf: &RouterConfig<A>,
        encoder: Rc<RwLock<IO>>,
    ) -> Result<bool, GError> {
        let name = conf.server_name.get_acme_path()?;
        let p = Path::new(&name);
        match &conf.tls {
            Some(_) => {
                let req_path = req.uri().path().to_string();
                log::info!("acme: request path: {}", req_path);
                if req_path.starts_with(ACME_URI_PREFIX) {
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
                            let _ = encoder.write().unwrap().send_and_flush(response).await;
                            info!("acme challenge replied");
                            return Ok(true);
                        }
                        Err(e) => {
                            log::warn!("find acme file error: {}", e);
                            let data = Bytes::from_static(b"404 not found --- Monoio Gateway.");
                            let response = Builder::new()
                                .body(Payload::Fixed(FixedPayload::new(data)))
                                .unwrap();
                            let _ = encoder.write().unwrap().send_and_flush(response).await;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(false)
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

/// handle backward connections and send request to endpoint.
/// This function use spawn feature of monoio and will not block caller.
async fn handle_endpoint_connection<O>(
    connect_pool: SharedTcpConnectPool<TcpStream, TcpStream>,
    proxy_pass: &Domain,
    encoder: Rc<RwLock<GenericEncoder<O>>>,
    mut request: Request<Payload>,
) where
    O: AsyncWriteRent + 'static,
{
    if !connect_pool.read().unwrap().contains_key(proxy_pass) {
        log::info!(
            "{} endpoint connections not exists, try connect now",
            proxy_pass
        );
        // open channel
        let proxy_pass_domain = proxy_pass.clone();
        let local_encoder_clone = encoder.clone();
        // no connections
        let mut connect_svc = ConnectEndpoint::default();
        // hold endpoint request, prevent
        let mut connect_pool_w = connect_pool.write().unwrap();
        if let Ok(Some(conn)) = connect_svc
            .call(EndpointRequestParams {
                endpoint: proxy_pass_domain.clone(),
            })
            .await
        {
            let conn = Rc::new(conn);
            connect_pool_w.insert(proxy_pass_domain.clone(), conn.clone());
            // endpoint -> proxy -> client
            let connect_pool_cloned = connect_pool.clone();
            monoio::spawn(async move {
                match conn.borrow() {
                    ClientConnectionType::Http(i, _) => {
                        let _ =
                            copy_response_lock(i.clone(), local_encoder_clone, &proxy_pass_domain)
                                .await;
                    }
                    ClientConnectionType::Tls(i, _) => {
                        let _ =
                            copy_response_lock(i.clone(), local_encoder_clone, &proxy_pass_domain)
                                .await;
                    }
                }
                // remove proxy pass endpoint
                connect_pool_cloned
                    .write()
                    .unwrap()
                    .remove(&proxy_pass_domain);
                log::info!("remove {} from endpoint pool", &proxy_pass_domain);
            });
        } else {
            // connect endpoint failed
            let _ = encoder
                .write()
                .unwrap()
                .send_and_flush(generate_response(StatusCode::NOT_FOUND))
                .await;
        }
    }
    if let Some(conn) = connect_pool.read().unwrap().get(proxy_pass) {
        // send this request to endpoint
        let conn = conn.clone();
        let proxy_pass_domain = proxy_pass.clone();
        monoio::spawn(async move {
            Rewrite::rewrite_request(&mut request, &proxy_pass_domain);
            match conn.borrow() {
                ClientConnectionType::Http(_, sender) => {
                    let _ = sender.write().unwrap().send_and_flush(request).await;
                }
                ClientConnectionType::Tls(_, sender) => {
                    let _ = sender.write().unwrap().send_and_flush(request).await;
                }
            }
        });
    }
}
