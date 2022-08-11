use std::{future::Future, net::SocketAddr};

use monoio::io::sink::SinkExt;
use monoio::net::TcpStream;
use monoio::{io::stream::Stream, net::TcpListener};

use monoio_gateway_core::config::ProxyConfig;
use monoio_gateway_core::dns::http::Domain;
use monoio_gateway_core::dns::Resolvable;
use monoio_gateway_core::transfer::copy_stream_sink;
use monoio_http::h1::codec::decoder::{FillPayload, ResponseDecoder};
use monoio_http::h1::codec::{decoder::RequestDecoder, encoder::GenericEncoder};

use super::Proxy;

pub type HttpProxyConfig = ProxyConfig<Domain>;

pub struct HttpProxy {
    config: HttpProxyConfig,
}

impl Proxy for HttpProxy {
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
                                    if let Err(decode_error) = local_dec.fill_payload().await {
                                        log::info!("{}", decode_error);
                                        continue;
                                    }
                                    let headers = req.headers();
                                    match headers.get("host") {
                                        None => {
                                            eprintln!("no host provided, ignore current request");
                                        }
                                        Some(host) => {
                                            let inbound = self.inbound_host();
                                            if host.to_str().unwrap_or("") == inbound {
                                                match TcpStream::connect(self.outbound_host()).await
                                                {
                                                    Ok(stream) => {
                                                        let (r_r, r_w) = stream.into_split();
                                                        let mut remote_dec =
                                                            ResponseDecoder::new(r_r);
                                                        let mut remote_enc =
                                                            GenericEncoder::new(r_w);

                                                        let _ =
                                                            remote_enc.send_and_flush(req).await;
                                                        let _ = monoio::join!(
                                                            copy_stream_sink(
                                                                &mut remote_dec,
                                                                &mut local_enc
                                                            ),
                                                            copy_stream_sink(
                                                                &mut local_dec,
                                                                &mut remote_enc
                                                            )
                                                        );
                                                    }
                                                    Err(e) => {
                                                        eprintln!("{}", e);
                                                    }
                                                }
                                            } else {
                                                // ignore current session
                                                eprintln!("ignore current session.");
                                            }
                                        }
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
                    eprintln!(
                        "bind error for http proxy with address {}, reason: {}",
                        bind_address, err
                    );
                    return Err(anyhow::anyhow!(
                        "error bind listener to {}",
                        self.listen_port(),
                    ));
                }
            }
        }
    }
}

impl HttpProxy {
    pub fn build_with_config(config: &HttpProxyConfig) -> Self {
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

    pub fn outbound_host(&self) -> String {
        let host = self.config.outbound.server.addr.host();
        if let Some(_) = str::find(&host, ':') {
            host.to_owned()
        } else {
            host.to_owned() + &format!(":{}", self.config.outbound.server.addr.port())
        }
    }

    pub fn inbound_host(&self) -> String {
        let host = self.config.inbound.server.addr.host();
        if let Some(_) = str::find(&host, ':') {
            host.to_owned()
        } else {
            host.to_owned() + &format!(":{}", self.config.inbound.server.addr.port())
        }
    }

    fn listen_port(&self) -> u16 {
        self.config.inbound.server.addr.port()
    }

    pub fn configure(&mut self) {}
}
