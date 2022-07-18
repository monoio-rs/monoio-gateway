use monoio::io::{AsyncReadRent, AsyncWriteRent, AsyncWriteRentExt};

pub mod h1;
pub mod h2;
pub mod tcp;

pub trait Proxy {}

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
