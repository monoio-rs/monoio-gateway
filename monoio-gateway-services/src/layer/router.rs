use std::{collections::HashMap, future::Future};

use anyhow::bail;

use log::{debug, info};
use monoio::io::stream::Stream;
use monoio_gateway_core::{
    dns::{http::Domain, Resolvable},
    error::GError,
    http::router::{RouterConfig, RouterRule},
    service::{Layer, Service},
};
use monoio_http::{
    common::request::Request,
    h1::{
        codec::{decoder::RequestDecoder, encoder::GenericEncoder},
        payload::Payload,
    },
};

use crate::layer::transfer::TransferParamsType;

use super::{accept::TcpAccept, endpoint::EndpointRequestParams, tls::TlsAccept};
#[derive(Clone)]
pub struct RouterService<T, A> {
    inner: T,
    routes: HashMap<String, RouterConfig<A>>,
}

impl<T> Service<TlsAccept> for RouterService<T, Domain>
where
    T: Service<EndpointRequestParams<Domain>>,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: TlsAccept) -> Self::Future<'_> {
        async {
            let (local_read, local_write) = req.0.split();
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
                                        match self
                                            .inner
                                            .call(EndpointRequestParams::new(
                                                TransferParamsType::ServerTls(
                                                    local_encoder,
                                                    local_decoder,
                                                ),
                                                proxy_pass.to_owned(),
                                                Some(req),
                                            ))
                                            .await
                                        {
                                            Ok(resp) => return Ok(Some(resp)),
                                            Err(err) => bail!("{}", err),
                                        }
                                    } else {
                                        // no match router rule
                                        info!("no matching router rule, {}", domain);
                                    }
                                }
                                None => {
                                    info!("no matching endpoint, ignoring {}", domain);
                                }
                            }
                        }
                        None => {
                            // no host, ignore!
                            info!("request has no host, uri: {}", req.uri());
                        }
                    }
                }
                Some(Err(err)) => {
                    // TODO: fallback to tcp
                    debug!("detect ssl failed, fallback to tcp");
                    bail!("{}", err)
                }
                _ => {}
            }
            Ok(None)
        }
    }
}

impl<T> Service<TcpAccept> for RouterService<T, Domain>
where
    T: Service<EndpointRequestParams<Domain>>,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, local_stream: TcpAccept) -> Self::Future<'_> {
        async move {
            debug!("find route for {:?}", local_stream.1);
            let (local_read, local_write) = local_stream.0.into_split();
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
                                                TransferParamsType::ServerHttp(
                                                    local_encoder,
                                                    local_decoder,
                                                ),
                                                proxy_pass.clone(),
                                                Some(req),
                                            ))
                                            .await
                                        {
                                            Ok(resp) => {
                                                return Ok(Some(resp));
                                            }
                                            Err(_) => bail!("endpoint communication failed"),
                                        }
                                    } else {
                                        // no match router rule
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

impl<T, A> RouterService<T, A>
where
    A: Resolvable,
{
    #[inline]
    fn match_target(&self, host: &String) -> Option<&RouterConfig<A>> {
        self.routes.get(host)
    }
}

pub struct RouterLayer<A> {
    routes: HashMap<String, RouterConfig<A>>,
}

impl<A> RouterLayer<A> {
    pub fn new(routes: HashMap<String, RouterConfig<A>>) -> Self {
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
