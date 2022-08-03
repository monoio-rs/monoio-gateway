use std::{future::Future, net::SocketAddr};
use std::fmt::Pointer;

use monoio::{
    io::stream::Stream,
    net::{TcpListener},
};
use monoio::io::sink::{Sink, SinkExt};
use monoio_http::common::request::Request;
use monoio_http::common::response::ResponseBuilder;
use monoio_http::h1::codec::{
    decoder::{RequestDecoder},
    encoder::GenericEncoder
};
use monoio_http::h1::payload::{FixedPayload, Payload};

use crate::{
    config::ProxyConfig,
    dns::{http::Domain, Resolvable},
};

use super::Proxy;

pub type HttpProxyConfig<'cx> = ProxyConfig<'cx, Domain>;

pub struct HttpProxy<'cx> {
    config: HttpProxyConfig<'cx>,
}

impl<'cx> Proxy for HttpProxy<'cx> {
    type Error = anyhow::Error;
    type OutputFuture<'a> = impl Future<Output = Result<(), Self::Error>> where Self: 'a;

    fn io_loop(&mut self) -> Self::OutputFuture<'_> {
        async {
            println!("start a http proxy");
            // start listen first
            let bind_address = format!("127.0.0.1:{}", self.listen_port());
            match TcpListener::bind(bind_address.to_owned()) {
                Ok(listener) => {
                    loop {
                        if let Ok((stream, addr)) = listener.accept().await {
                            println!("accept address: {}", addr);
                            let (r, w) = stream.into_split();

                            let mut local_dec = RequestDecoder::new(r);
                            let mut local_enc = GenericEncoder::new(w);
                            // demo mode
                            match local_dec.next().await {
                                Some(Ok(req)) => {
                                    println!("{:?}", req.().host());
                                    if let Some(req_host) = req.uri().host() {
                                        println!("host: {}", req_host);
                                        let resp = ResponseBuilder::default().body(
                                            Payload::from(FixedPayload::new("hello".into()))
                                        ).unwrap();
                                        local_enc.send_and_flush(resp).await?;
                                    }
                                }
                                Some(Err(e)) => {
                                    eprintln!("http decode error: {:?}", e);
                                }
                                None => {
                                    println!("no decode data");
                                }
                            }
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    eprintln!("bind error for http proxy with address {}, reason: {}", bind_address, err);
                    return Err(anyhow::anyhow!(
                    "error bind listener to {}",
                    self.listen_port(),
                ));
                }
            }
        }
    }
}

impl<'cx> HttpProxy<'cx> {
    pub fn build_with_config(config: &HttpProxyConfig<'cx>) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn inbound_addr(&self) -> Result<SocketAddr, anyhow::Error> {
        let resolved = self.config.inbound.server.addr.resolve().await?;
        if let Some(res) = resolved {
            Ok(res)
        } else {
            Err(anyhow::anyhow!("resolve http inbound addr failed."))
        }
    }

    pub async fn outbound_addr(&self) -> Result<SocketAddr, anyhow::Error> {
        let resolved = self.config.outbound.server.addr.resolve().await?;
        if let Some(res) = resolved {
            Ok(res)
        } else {
            Err(anyhow::anyhow!("resolve http outbound addr failed."))
        }
    }

    fn listen_port(&self) -> u16 {
        self.config.inbound.server.addr.port()
    }

    pub fn configure(&mut self) {}
}
