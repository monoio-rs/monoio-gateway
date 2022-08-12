use std::future::Future;

use monoio::net::TcpStream;
use monoio_gateway_core::{error::GError, service::Service, transfer::copy_data};

#[derive(Default)]
pub struct TransferService;

pub type TransferParams = (TcpStream, TcpStream);

impl Service<TransferParams> for TransferService {
    type Response = ();

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
    where
        Self: 'cx;

    fn call(&mut self, req: TransferParams) -> Self::Future<'_> {
        async {
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

// impl Service<Request> for TransferService {
//     type Response = ();

//     type Error = GError;

//     type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>>
//     where
//         Self: 'cx;

//     fn call(&mut self, _req: Request) -> Self::Future<'_> {
//         async {
//             let (mut local_read, mut local_write) = self.local_io.split();
//             let (mut remote_read, mut remote_write) = self.remote_io.split();
//             let _ = monoio::join!(
//                 copy_data(&mut local_read, &mut remote_write),
//                 copy_data(&mut remote_read, &mut local_write)
//             );
//             Ok(())
//         }
//     }
// }
