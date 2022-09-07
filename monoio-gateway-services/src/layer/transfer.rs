use std::future::Future;

use log::info;
use monoio::{
    io::{sink::SinkExt, AsyncReadRent, AsyncWriteRent, OwnedReadHalf, OwnedWriteHalf, Splitable},
    net::TcpStream,
};
use monoio_gateway_core::{
    dns::{http::Domain, Resolvable},
    error::GError,
    service::Service,
    transfer::{copy_data, copy_request, copy_response},
};
use monoio_http::{
    common::request::Request,
    h1::codec::{
        decoder::{RequestDecoder, ResponseDecoder},
        encoder::GenericEncoder,
    },
};

#[derive(Default, Clone)]
pub struct HttpTransferService;

#[derive(Default, Clone)]
pub struct TcpTransferService;

pub type TcpTransferParams = (TcpStream, TcpStream);

pub struct TransferParams<
    L: AsyncWriteRent + AsyncReadRent,
    R: AsyncWriteRent + AsyncReadRent,
    A: Resolvable,
> {
    local: TransferParamsType<A, L>,  // client
    remote: TransferParamsType<A, R>, // server
    pub(crate) local_req: Option<Request>,
}

impl<L: AsyncWriteRent + AsyncReadRent, R: AsyncWriteRent + AsyncReadRent, A: Resolvable>
    TransferParams<L, R, A>
{
    pub fn new(
        local: TransferParamsType<A, L>,
        remote: TransferParamsType<A, R>,
        local_req: Option<Request>,
    ) -> Self {
        Self {
            local,
            remote,
            local_req,
        }
    }
}

pub enum TransferParamsType<A, S>
where
    S: AsyncWriteRent,
{
    ServerTls(
        GenericEncoder<monoio_rustls::ServerTlsStreamWriteHalf<S>>,
        RequestDecoder<monoio_rustls::ServerTlsStreamReadHalf<S>>,
        A,
    ),
    ClientTls(
        GenericEncoder<monoio_rustls::ClientTlsStreamWriteHalf<S>>,
        ResponseDecoder<monoio_rustls::ClientTlsStreamReadHalf<S>>,
        A,
    ),

    ServerHttp(
        GenericEncoder<OwnedWriteHalf<S>>,
        RequestDecoder<OwnedReadHalf<S>>,
        A,
    ),
    ClientHttp(
        GenericEncoder<OwnedWriteHalf<S>>,
        ResponseDecoder<OwnedReadHalf<S>>,
        A,
    ),
}

impl Service<TcpTransferParams> for TcpTransferService {
    type Response = ();

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: TcpTransferParams) -> Self::Future<'_> {
        async {
            info!("transfer data");
            let mut local_io = req.0;
            let mut remote_io = req.1;
            let (mut local_read, mut local_write) = local_io.split();
            let (mut remote_read, mut remote_write) = remote_io.split();
            let _ = monoio::join!(
                copy_data(&mut local_read, &mut remote_write),
                copy_data(&mut remote_read, &mut local_write)
            );
            Ok(())
        }
    }
}

impl<L, R> Service<TransferParams<L, R, Domain>> for HttpTransferService
where
    L: AsyncWriteRent + AsyncReadRent,
    R: AsyncWriteRent + AsyncReadRent,
{
    type Response = ();

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: TransferParams<L, R, Domain>) -> Self::Future<'_> {
        async {
            info!("transfering data");
            match req.local {
                TransferParamsType::ServerTls(mut lw, mut lr, local) => match req.remote {
                    TransferParamsType::ClientTls(mut rw, mut rr, remote) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await?;
                        }
                        monoio::select! {
                            _ = copy_request(&mut lr, &mut rw, &remote) => {
                                return Ok(());
                            }
                            _ = copy_response(&mut rr, &mut lw, &local) => {
                                return Ok(());
                            }
                        }
                    }
                    TransferParamsType::ClientHttp(mut rw, mut rr, remote) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await?;
                        }
                        monoio::select! {
                            _ = copy_request(&mut lr, &mut rw, &remote) => {
                                return Ok(());
                            }
                            _ = copy_response(&mut rr, &mut lw, &local) => {
                                return Ok(());
                            }
                        }
                    }
                    _ => {}
                },
                TransferParamsType::ServerHttp(mut lw, mut lr, local) => match req.remote {
                    TransferParamsType::ClientTls(mut rw, mut rr, remote) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await?;
                        }
                        monoio::select! {
                            _ = copy_request(&mut lr, &mut rw, &remote) => {
                                return Ok(());
                            }
                            _ = copy_response(&mut rr, &mut lw, &local) => {
                                return Ok(());
                            }
                        }
                    }
                    TransferParamsType::ClientHttp(mut rw, mut rr, remote) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await?;
                        }
                        monoio::select! {
                            _ = copy_request(&mut lr, &mut rw, &remote) => {
                                return Ok(());
                            }
                            _ = copy_response(&mut rr, &mut lw, &local) => {
                                return Ok(());
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
            Ok(())
        }
    }
}
