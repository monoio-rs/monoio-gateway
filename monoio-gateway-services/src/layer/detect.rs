use std::future::Future;

use monoio::{
    io::{AsyncReadRent, PrefixedReadIo},
    net::TcpStream,
};
use monoio_gateway_core::{
    error::GError,
    http::{detect::DetectHttpVersion, version::Type, Detect},
    service::Service,
};

use super::accept::TcpAccept;

const SSL_RECORD_TYPE: u8 = 22;
const SSL: u8 = 0x03;

#[derive(Clone)]
pub struct DetectService;

pub type DetectResult = (Type, PrefixedReadIo<TcpStream, Vec<u8>>);

pub struct DetectResponse<I, P> {
    pub pio: PrefixedReadIo<I, P>,
}

impl Service<TcpAccept> for DetectService {
    type Response = Option<DetectResult>;

    type Error = GError;

    type Future<'cx> = impl Future<Output = Result<Self::Response, GError>> where Self: 'cx;

    fn call(&mut self, acc: TcpAccept) -> Self::Future<'_> {
        // let detect = self.detect.clone();
        // Byte   0       = SSL record type
        // Bytes 1-2      = SSL version (major/minor)
        // Bytes 3-4      = Length of data in the record (excluding the header itself).
        //                  The maximum SSL supports is 16384 (16K).
        // we use first 3 bytes to read
        let buf = vec![0 as u8; 3];
        async move {
            let (mut tcp, _) = acc;
            let (sz, buf) = tcp.read(buf).await;
            let _sz = sz?;
            let ssl_record_type: u8 = buf[0];
            let ssl_version_b1: u8 = buf[1];
            // TODO: add ssl version detect
            let _ssl_version_b2: u8 = buf[2];
            let pio = PrefixedReadIo::new(tcp, buf);
            // 22 -> SSL
            if ssl_record_type != SSL_RECORD_TYPE {
                return Ok(Some((Type::HTTP, pio)));
            }
            if ssl_version_b1 != SSL {
                return Ok(Some((Type::HTTP, pio)));
            }
            Ok(Some((Type::HTTPS, pio)))
        }
    }
}

impl DetectService {
    pub fn new_http_detect() -> Self {
        Self
    }
}
