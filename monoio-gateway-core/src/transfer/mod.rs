use log::info;
use monoio::{
    io::{
        sink::Sink, stream::Stream, AsyncReadRent, AsyncWriteRent, AsyncWriteRentExt,
        PrefixedReadIo,
    },
    net::TcpStream,
};
use monoio_http::{
    common::IntoParts,
    h1::codec::decoder::{DecodeError, FillPayload},
};

pub type TcpPrefixedIo = PrefixedReadIo<TcpStream, Vec<u8>>;

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
where
    Read: Stream<Item = Result<I, DecodeError>> + FillPayload,
    Write: Sink<I>,
    I: IntoParts,
{
    loop {
        match local.next().await {
            Some(Ok(data)) => {
                info!("sending data");
                let _ = local.fill_payload().await;
                let _ = remote.send(data).await;
                let _ = monoio::io::sink::Sink::flush(remote).await;
                info!("data sent");
            }
            Some(Err(decode_error)) => {
                eprintln!("{}", decode_error);
            }
            None => {
                info!("reached EOF");
                break;
            }
        }
    }
    Ok(())
}
