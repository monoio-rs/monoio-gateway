use std::future::Future;

use monoio::{io::AsyncWriteRentExt, net::TcpStream};
use monoio_gateway_core::{error::GError, service::Service, transfer::copy_data};
use monoio_http::common::request::Request;

pub struct TransferService<I, O> {
    local_io: I,
    remote_io: O,
}

impl Service<Vec<u8>> for TransferService<TcpStream, TcpStream> {
    type Response = ();

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: Vec<u8>) -> Self::Future<'_> {
        async {
            let (mut local_read, mut local_write) = self.local_io.split();
            let (mut remote_read, mut remote_write) = self.remote_io.split();
            let _ = remote_write.write_all(req);
            let _ = monoio::join!(
                copy_data(&mut local_read, &mut remote_write),
                copy_data(&mut remote_read, &mut local_write)
            );
            Ok(())
        }
    }
}

impl Service<Request> for TransferService<TcpStream, TcpStream> {
    type Response = ();

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, _req: Request) -> Self::Future<'_> {
        async {
            let (mut local_read, mut local_write) = self.local_io.split();
            let (mut remote_read, mut remote_write) = self.remote_io.split();
            let _ = monoio::join!(
                copy_data(&mut local_read, &mut remote_write),
                copy_data(&mut remote_read, &mut local_write)
            );
            Ok(())
        }
    }
}
