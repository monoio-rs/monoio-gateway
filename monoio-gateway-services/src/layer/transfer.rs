use std::future::Future;

use log::info;
use monoio::{
    io::sink::SinkExt,
    net::{
        tcp::{TcpOwnedReadHalf, TcpOwnedWriteHalf},
        TcpStream,
    },
};
use monoio_gateway_core::{
    error::GError,
    service::Service,
    transfer::{copy_data, copy_stream_sink},
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

pub struct TransferParams {
    local: TransferParamsType,  // client
    remote: TransferParamsType, // server
    local_req: Option<Request>,
}

impl TransferParams {
    pub fn new(
        local: TransferParamsType,
        remote: TransferParamsType,
        local_req: Option<Request>,
    ) -> Self {
        Self {
            local,
            remote,
            local_req,
        }
    }
}

pub enum TransferParamsType {
    ServerTls(
        GenericEncoder<monoio_rustls::ServerTlsStreamWriteHalf<TcpStream>>,
        RequestDecoder<monoio_rustls::ServerTlsStreamReadHalf<TcpStream>>,
    ),
    ClientTls(
        GenericEncoder<monoio_rustls::ClientTlsStreamWriteHalf<TcpStream>>,
        ResponseDecoder<monoio_rustls::ClientTlsStreamReadHalf<TcpStream>>,
    ),

    ServerHttp(
        GenericEncoder<TcpOwnedWriteHalf>,
        RequestDecoder<TcpOwnedReadHalf>,
    ),
    ClientHttp(
        GenericEncoder<TcpOwnedWriteHalf>,
        ResponseDecoder<TcpOwnedReadHalf>,
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

impl Service<TransferParams> for HttpTransferService {
    type Response = ();

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: TransferParams) -> Self::Future<'_> {
        async {
            info!("transfering data");
            match req.local {
                TransferParamsType::ServerTls(mut lw, mut lr) => match req.remote {
                    TransferParamsType::ClientTls(mut rw, mut rr) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await;
                        }
                        let _ = monoio::join!(
                            copy_stream_sink(&mut lr, &mut rw),
                            copy_stream_sink(&mut rr, &mut lw)
                        );
                    }
                    TransferParamsType::ClientHttp(mut rw, mut rr) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await;
                        }
                        let _ = monoio::join!(
                            copy_stream_sink(&mut lr, &mut rw),
                            copy_stream_sink(&mut rr, &mut lw)
                        );
                    }
                    _ => {}
                },
                TransferParamsType::ServerHttp(mut lw, mut lr) => match req.remote {
                    TransferParamsType::ClientTls(mut rw, mut rr) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await;
                        }
                        let _ = monoio::join!(
                            copy_stream_sink(&mut lr, &mut rw),
                            copy_stream_sink(&mut rr, &mut lw)
                        );
                    }
                    TransferParamsType::ClientHttp(mut rw, mut rr) => {
                        if let Some(request) = req.local_req {
                            rw.send_and_flush(request).await;
                        }
                        let _ = monoio::join!(
                            copy_stream_sink(&mut lr, &mut rw),
                            copy_stream_sink(&mut rr, &mut lw)
                        );
                    }
                    _ => {}
                },
                _ => {}
            }
            Ok(())
        }
    }
}
