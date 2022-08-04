use std::fmt::{Debug, Display};
use std::future::Future;

use monoio::io::{AsyncReadRent, AsyncWriteRent, AsyncWriteRentExt};
use monoio::io::sink::{Sink, SinkExt};
use monoio::io::stream::Stream;
use monoio_http::common::IntoParts;
use monoio_http::h1::codec::decoder::DecodeError;
use monoio_http::h1::payload::Payload;

pub mod h1;
pub mod h2;
pub mod tcp;

pub trait Proxy {
    type Error;
    type OutputFuture<'a>: Future<Output = Result<(), Self::Error> >
    where
    Self: 'a;

    fn io_loop(&mut self) -> Self::OutputFuture<'_>;
}

pub async fn copy_data<Read: AsyncReadRent, Write: AsyncWriteRent>(
    local: &mut Read,
    remote: &mut Write,
) -> Result<Vec<u8>, std::io::Error> {
    let mut buf = vec![0; 1024];
    loop {
        let (res, read_buffer) = local.read(buf).await;
        buf = read_buffer;
        let read_len = res?;
        if read_len == 0 {
            // no
            return Ok(buf);
        }
        // write to remote
        let (res, write_buffer) = remote.write_all(buf).await;
        buf = write_buffer;
        let _ = res?;
        buf.clear();
    }
}


pub async fn copy_stream_sink<I, Read, Write>(
    local: &mut Read,
    remote: &mut Write,
) -> Result<(), std::io::Error>
    where Read: Stream<Item=Result<I, DecodeError>>,
          Write: Sink<I>,
          I: IntoParts
{
    loop {
        match local.next().await {
            Some(Ok(data)) => {
                let _ = remote.send(data).await;
                let _ = remote.flush().await;
            }
            Some(Err(decode_error)) => {
                eprintln!("{}", decode_error);
            }
            None => {
                break;
            }
        }
    }
    Ok(())
}
