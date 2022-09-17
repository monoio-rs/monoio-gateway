use std::{future::Future, rc::Rc, sync::RwLock};

use anyhow::bail;
use log::info;
use monoio::{
    io::{AsyncWriteRent, OwnedReadHalf, OwnedWriteHalf, Splitable},
    net::TcpStream,
};
use monoio_gateway_core::{
    dns::{http::Domain, Resolvable},
    error::GError,
    http::ssl::get_default_tls_connector,
    service::Service,
};
use monoio_http::h1::codec::{decoder::ResponseDecoder, encoder::GenericEncoder};

use rustls::ServerName;

pub struct EndpointRequestParams<EndPoint> {
    pub(crate) endpoint: EndPoint,
}

impl<Endpoint> EndpointRequestParams<Endpoint> {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }
}

#[derive(Default, Clone)]
pub struct ConnectEndpoint;

pub enum ClientConnectionType<I, O: AsyncWriteRent> {
    Http(
        Rc<RwLock<ResponseDecoder<OwnedReadHalf<I>>>>,
        Rc<RwLock<GenericEncoder<OwnedWriteHalf<O>>>>,
    ),
    Tls(
        Rc<RwLock<ResponseDecoder<monoio_rustls::ClientTlsStreamReadHalf<I>>>>,
        Rc<RwLock<GenericEncoder<monoio_rustls::ClientTlsStreamWriteHalf<O>>>>,
    ),
}

impl Service<EndpointRequestParams<Domain>> for ConnectEndpoint {
    type Response = Option<ClientConnectionType<TcpStream, TcpStream>>;

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
                                // no need to handshake
                                let (r, w) = stream.into_split();
                                return Ok(Some(ClientConnectionType::Http(
                                    Rc::new(RwLock::new(ResponseDecoder::new(r))),
                                    Rc::new(RwLock::new(GenericEncoder::new(w))),
                                )));
                            }
                            monoio_gateway_core::http::version::Type::HTTPS => {
                                info!("establishing https connection to endpoint");
                                let tls_connector = get_default_tls_connector();
                                let server_name =
                                    ServerName::try_from(req.endpoint.host().as_ref())?;
                                match tls_connector.connect(server_name, stream).await {
                                    Ok(endpoint_stream) => {
                                        let (r, w) = endpoint_stream.split();
                                        return Ok(Some(ClientConnectionType::Tls(
                                            Rc::new(RwLock::new(ResponseDecoder::new(r))),
                                            Rc::new(RwLock::new(GenericEncoder::new(w))),
                                        )));
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
