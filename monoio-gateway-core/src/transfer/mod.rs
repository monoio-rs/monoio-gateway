use std::{cell::UnsafeCell, rc::Rc};

use http::{header::HOST, StatusCode};
use monoio::{
    io::{
        sink::{Sink, SinkExt},
        stream::Stream,
        AsyncReadRent, AsyncWriteRent, AsyncWriteRentExt, PrefixedReadIo,
    },
    net::TcpStream,
};
use monoio_http::{
    common::{request::Request, response::Response, IntoParts},
    h1::{
        codec::decoder::{DecodeError, FillPayload},
        payload::Payload,
    },
};

use crate::{dns::http::Domain, http::Rewrite};

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
    I: IntoParts + 'static,
{
    loop {
        match local.next().await {
            Some(Ok(data)) => {
                log::debug!("sending data");
                let _ = monoio::join!(local.fill_payload(), remote.send_and_flush(data));
                log::debug!("data sent");
            }
            Some(Err(decode_error)) => {
                log::warn!("DecodeError: {}", decode_error);
            }
            None => {
                log::info!("reached EOF, bye");
                let _ = remote.close().await;
                break;
            }
        }
    }
    Ok(())
}

pub async fn copy_request<Read, Write>(
    local: &mut Read,
    remote: &mut Write,
    domain: &Domain,
) -> Result<(), std::io::Error>
where
    Read: Stream<Item = Result<Request<Payload>, DecodeError>> + FillPayload,
    Write: Sink<Request<Payload>>,
{
    loop {
        match local.next().await {
            Some(Ok(request)) => {
                let mut request: Request = request;
                Rewrite::rewrite_request(&mut request, domain);
                log::info!(
                    "request: {}, host: {:?}",
                    request.uri(),
                    request.headers().get(HOST)
                );
                let _ = monoio::join!(local.fill_payload(), remote.send_and_flush(request));
                log::debug!("request sent");
            }
            Some(Err(decode_error)) => {
                log::warn!("Decode Error: {}", decode_error);
            }
            None => {
                log::info!("forward reached EOF, bye");
                let _ = remote.close().await;
                break;
            }
        }
    }
    Ok(())
}

pub async fn copy_response<Read, Write>(
    local: &mut Read,
    remote: &mut Write,
    domain: &Domain,
) -> Result<(), std::io::Error>
where
    Read: Stream<Item = Result<Response<Payload>, DecodeError>> + FillPayload,
    Write: Sink<Response<Payload>>,
{
    loop {
        match local.next().await {
            Some(Ok(response)) => {
                let mut response: Response = response;
                Rewrite::rewrite_response(&mut response, domain);
                log::info!(
                    "response code: {},{:?}",
                    response.status(),
                    response.headers(),
                );
                let _ = monoio::join!(local.fill_payload(), remote.send_and_flush(response));
            }
            Some(Err(decode_error)) => {
                log::warn!("DecodeError: {}", decode_error);
            }
            None => {
                log::info!("backward reached EOF, bye");
                let _ = remote.close().await;
                break;
            }
        }
    }
    Ok(())
}

pub async fn copy_response_lock<Read, Write>(
    local: Rc<UnsafeCell<Read>>,
    remote: Rc<UnsafeCell<Write>>,
    domain: Domain,
) -> Result<(), std::io::Error>
where
    Read: Stream<Item = Result<Response<Payload>, DecodeError>> + FillPayload,
    Write: Sink<Response<Payload>>,
{
    let local = unsafe { &mut *local.get() };
    let remote = unsafe { &mut *remote.get() };
    loop {
        match local.next().await {
            Some(Ok(response)) => {
                let mut response: Response = response;
                Rewrite::rewrite_response(&mut response, &domain);
                log::info!(
                    "response code: {},{:?}",
                    response.status(),
                    response.headers(),
                );
                let _ = monoio::join!(local.fill_payload(), remote.send_and_flush(response));
            }
            Some(Err(decode_error)) => {
                log::warn!("DecodeError: {}", decode_error);
                break;
            }
            None => {
                log::info!("backward reached EOF, bye");
                break;
            }
        }
    }
    let _ = remote.close().await;
    Ok(())
}

pub fn generate_response(status_code: StatusCode) -> Response {
    let mut resp = Response::builder();
    resp = resp.status(status_code);
    resp.body(Payload::None).unwrap()
}
