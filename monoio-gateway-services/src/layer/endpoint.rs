use std::future::Future;

use anyhow::bail;
use log::info;
use monoio::net::TcpStream;
use monoio_gateway_core::{
    dns::{http::Domain, Resolvable},
    error::GError,
    service::{Layer, Service},
};
use monoio_http::{
    common::request::Request,
    h1::codec::{decoder::ResponseDecoder, encoder::GenericEncoder},
};

use rustls::ServerName;

use super::{
    tls::get_default_tls_connector,
    transfer::{TransferParams, TransferParamsType},
};

pub struct EndpointRequestParams<EndPoint> {
    local: TransferParamsType,
    local_req: Option<Request>,
    endpoint: EndPoint,
}

impl<Endpoint> EndpointRequestParams<Endpoint> {
    pub fn new(local: TransferParamsType, endpoint: Endpoint, local_req: Option<Request>) -> Self {
        Self {
            local,
            endpoint,
            local_req,
        }
    }
}

#[derive(Clone)]
pub struct ConnectEndpoint<T> {
    inner: T,
}

impl<T> Service<EndpointRequestParams<Domain>> for ConnectEndpoint<T>
where
    T: Service<TransferParams>,
{
    type Response = Option<T::Response>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: EndpointRequestParams<Domain>) -> Self::Future<'_> {
        async move {
            info!("trying to connect to endpoint");
            let resolved = req.endpoint.resolve().await?;
            match resolved {
                Some(addr) => {
                    info!("resolved addr: {}", addr);
                    match TcpStream::connect(addr).await {
                        Ok(stream) => match req.endpoint.version() {
                            monoio_gateway_core::http::version::Type::HTTP => {
                                info!("establishing http connection to endpoint");
                                let (rr, rw) = stream.into_split();
                                let (rr_decoder, rw_encoder) =
                                    (ResponseDecoder::new(rr), GenericEncoder::new(rw));
                                match self
                                    .inner
                                    .call(TransferParams::new(
                                        req.local,
                                        TransferParamsType::ClientHttp(rw_encoder, rr_decoder),
                                        req.local_req,
                                    ))
                                    .await
                                {
                                    Ok(resp) => return Ok(Some(resp)),
                                    Err(err) => bail!("{}", err),
                                }
                            }
                            monoio_gateway_core::http::version::Type::HTTPS => {
                                info!("establishing https connection to endpoint");
                                let tls_connector = get_default_tls_connector();
                                let server_name =
                                    ServerName::try_from(req.endpoint.host().as_ref())?;
                                match tls_connector.connect(server_name, stream).await {
                                    Ok(endpoint_stream) => {
                                        let (rr, rw) = endpoint_stream.split();
                                        let (rr_decoder, rw_encoder) =
                                            (ResponseDecoder::new(rr), GenericEncoder::new(rw));
                                        match self
                                            .inner
                                            .call(TransferParams::new(
                                                req.local,
                                                TransferParamsType::ClientTls(
                                                    rw_encoder, rr_decoder,
                                                ),
                                                req.local_req,
                                            ))
                                            .await
                                        {
                                            Ok(resp) => return Ok(Some(resp)),
                                            Err(err) => bail!("{}", err),
                                        };
                                    }
                                    Err(tls_error) => bail!("{}", tls_error),
                                }
                            }
                        },
                        Err(err) => bail!("error connect endpoint: {}", err),
                    }
                }
                _ => {}
            }
            Ok(None)
        }
    }
}

pub struct ConnectEndpointLayer;

impl ConnectEndpointLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for ConnectEndpointLayer {
    type Service = ConnectEndpoint<S>;

    fn layer(&self, service: S) -> Self::Service {
        ConnectEndpoint { inner: service }
    }
}
