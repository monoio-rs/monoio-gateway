use std::{borrow::Borrow, cell::UnsafeCell, collections::HashMap, future::Future, rc::Rc};

use async_channel::Receiver;
use http::StatusCode;
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
        payload::Payload,
    },
};

use crate::layer::endpoint::ConnectEndpoint;

use super::{
    accept::Accept,
    endpoint::{ClientConnectionType, EndpointRequestParams},
    tls::TlsAccept,
};

pub type SharedTcpConnectPool<I, O> =
    Rc<UnsafeCell<HashMap<String, Rc<ClientConnectionType<I, O>>>>>;

pub struct RouterService<A, I, O: AsyncWriteRent> {
    routes: Rc<RouterConfig<A>>,

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

    type Future<'a> = impl Future<Output = Result<Self::Response, Self::Error>> + 'a
    where
        Self: 'a;

    fn call(&mut self, local_stream: Accept<S>) -> Self::Future<'_> {
        async move {
            let (stream, socketaddr) = local_stream;
            let (local_read, local_write) = stream.into_split();
            let mut local_decoder = RequestDecoder::new(local_read);
            let local_encoder = Rc::new(UnsafeCell::new(GenericEncoder::new(local_write)));
            let (tx, rx) = async_channel::bounded(1);
            loop {
                let connect_pool = self.connect_pool.clone();
                match local_decoder.next().await {
                    Some(Ok(req)) => {
                        let req: Request<Payload> = req;

                        let m = longest_match(req.uri().path(), self.routes.get_rules());
                        if let Some(rule) = m {
                            // parsed rule for this request and spawn task to handle endpoint connection
                            let proxy_pass = rule.get_proxy_pass().to_owned();
                            handle_endpoint_connection(
                                connect_pool,
                                &proxy_pass,
                                local_encoder.clone(),
                                req,
                                rx.clone(),
                            )
                            .await;
                            continue;
                        } else {
                            debug!("no matching router rule for request uri:  {}", req.uri());
                            handle_request_error(local_encoder.clone(), StatusCode::NOT_FOUND)
                                .await;
                        }
                    }
                    Some(Err(err)) => {
                        // TODO: fallback to tcp
                        log::warn!("{}", err);
                        break;
                    }
                    None => {
                        info!("http client {} closed", socketaddr);
                        break;
                    }
                }
            }
            // notify disconnect from endpoints
            rx.close();
            let _ = tx.send(()).await;
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

    type Future<'a> = impl Future<Output = Result<Self::Response, Self::Error>> + 'a
    where
        Self: 'a;

    fn call(&mut self, local_stream: TlsAccept<S>) -> Self::Future<'_> {
        async move {
            let (stream, socketaddr, _) = local_stream;
            let (local_read, local_write) = stream.split();
            let mut local_decoder = RequestDecoder::new(local_read);
            let local_encoder = Rc::new(UnsafeCell::new(GenericEncoder::new(local_write)));
            // exit notifier
            let (tx, rx) = async_channel::bounded(1);
            loop {
                let connect_pool = self.connect_pool.clone();
                match local_decoder.next().await {
                    Some(Ok(req)) => {
                        let req: Request<Payload> = req;

                        let m = longest_match(req.uri().path(), self.routes.get_rules());
                        if let Some(rule) = m {
                            // parsed rule for this request and spawn task to handle endpoint connection
                            let proxy_pass = rule.get_proxy_pass().to_owned();
                            handle_endpoint_connection(
                                connect_pool,
                                &proxy_pass,
                                local_encoder.clone(),
                                req,
                                rx.clone(),
                            )
                            .await;
                            continue;
                        } else {
                            debug!("no matching router rule for request uri: {}", req.uri());
                            handle_request_error(local_encoder.clone(), StatusCode::NOT_FOUND)
                                .await;
                        }
                    }
                    Some(Err(err)) => {
                        log::warn!("{}", err);
                        break;
                    }
                    None => {
                        info!("https client {} closed", socketaddr);
                        break;
                    }
                }
            }
            log::info!("bye {}! Now we remove router", socketaddr);
            // notify disconnect from endpoints
            rx.close();
            let _ = tx.send(()).await;
            Ok(())
        }
    }
}

impl<A, I, O> RouterService<A, I, O>
where
    A: Resolvable,
    O: AsyncWriteRent,
{
    pub fn new(routes: Rc<RouterConfig<A>>) -> Self {
        Self {
            routes,
            connect_pool: Default::default(),
        }
    }
}

#[inline]
fn longest_match<'cx>(
    req_path: &'cx str,
    routes: &'cx Vec<RouterRule<Domain>>,
) -> Option<&'cx RouterRule<Domain>> {
    log::info!("request path: {}", req_path);
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

#[cfg(dead_code)]
#[inline]
fn get_host(req: &Request<Payload>) -> Option<&str> {
    match req.headers().get("host") {
        Some(host) => Some(host.to_str().unwrap_or("")),
        None => None,
    }
}

async fn handle_request_error<O>(encoder: Rc<UnsafeCell<GenericEncoder<O>>>, status: StatusCode)
where
    O: AsyncWriteRent + 'static,
    GenericEncoder<O>: monoio::io::sink::Sink<Response<Payload>>,
{
    let encoder = unsafe { &mut *encoder.get() };
    let _ = encoder.send_and_flush(generate_response(status)).await;
    let _ = encoder.close().await;
}

/// handle backward connections and send request to endpoint.
/// This function use spawn feature of monoio and will not block caller.
async fn handle_endpoint_connection<O>(
    connect_pool: SharedTcpConnectPool<TcpStream, TcpStream>,
    proxy_pass: &Domain,
    encoder: Rc<UnsafeCell<GenericEncoder<O>>>,
    mut request: Request<Payload>,
    rx: Receiver<()>,
) where
    O: AsyncWriteRent + 'static,
    GenericEncoder<O>: monoio::io::sink::Sink<Response<Payload>>,
{
    // we add a write lock to prevent multiple context execute into block below.
    {
        let connect_pool = unsafe { &mut *connect_pool.get() };
        // critical code start
        if !connect_pool.contains_key(proxy_pass.host()) {
            // hold endpoint request, prevent
            log::info!(
                "{} endpoint connections not exists, try connect now. [{:?}]",
                proxy_pass.host(),
                connect_pool.keys()
            );
            // open channel
            let proxy_pass_domain = proxy_pass.clone();
            let local_encoder_clone = encoder.clone();
            // no connections
            let mut connect_svc = ConnectEndpoint::default();
            if let Ok(Some(conn)) = connect_svc
                .call(EndpointRequestParams {
                    endpoint: proxy_pass.clone(),
                })
                .await
            {
                let conn = Rc::new(conn);
                connect_pool.insert(proxy_pass_domain.host().to_owned(), conn.clone());
                // endpoint -> proxy -> client
                let _connect_pool_cloned = connect_pool.clone();
                let rx_clone = rx.clone();
                monoio::spawn(async move {
                    match conn.borrow() {
                        ClientConnectionType::Http(i, _) => {
                            let cloned = local_encoder_clone.clone();
                            monoio::select! {
                                _ = copy_response_lock(i.clone(), local_encoder_clone, proxy_pass_domain.clone()) => {}
                                _ = rx_clone.recv() => {
                                    log::info!("client exit, now cancelling endpoint connection");
                                    handle_request_error(cloned, StatusCode::INTERNAL_SERVER_ERROR).await;
                                }
                            };
                        }
                        ClientConnectionType::Tls(i, _) => {
                            let cloned = local_encoder_clone.clone();
                            monoio::select! {
                                _ = copy_response_lock(i.clone(), local_encoder_clone, proxy_pass_domain.clone()) => {}
                                _ = rx_clone.recv() => {
                                    log::info!("client exit, now cancelling endpoint connection");
                                    handle_request_error(cloned, StatusCode::INTERNAL_SERVER_ERROR).await;
                                }
                            };
                        }
                    }
                    // remove proxy pass endpoint
                    connect_pool.remove(proxy_pass_domain.host());
                    log::info!("🗑 remove {} from endpoint pool", &proxy_pass_domain);
                });
            } else {
                handle_request_error(encoder, StatusCode::NOT_FOUND).await;
            }
        } else {
            log::info!("🚀 endpoint connection found for {}!", proxy_pass);
        }
    }
    let connect_pool = unsafe { &mut *connect_pool.get() };
    if let Some(conn) = connect_pool.get(proxy_pass.host()) {
        // send this request to endpoint
        let conn = conn.clone();
        let proxy_pass_domain = proxy_pass.clone();
        monoio::spawn(async move {
            Rewrite::rewrite_request(&mut request, &proxy_pass_domain);
            match conn.borrow() {
                ClientConnectionType::Http(_, sender) => {
                    let sender = unsafe { &mut *sender.get() };
                    let _ = sender.send_and_flush(request).await;
                }
                ClientConnectionType::Tls(_, sender) => {
                    let sender = unsafe { &mut *sender.get() };
                    let _ = sender.send_and_flush(request).await;
                }
            }
        });
    }
}
