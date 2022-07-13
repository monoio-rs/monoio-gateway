use std::{time::Duration, vec};

use monoio::{
    io::{AsyncReadRent, AsyncWriteRent, AsyncWriteRentExt},
    net::{TcpListener, TcpStream},
};

/// a simple tcp proxy
#[monoio::main(timer_enabled = true)]
async fn main() {
    const LISTEN_ADDRESS: &str = "127.0.0.1:9000";
    let fu = monoio::spawn(async move {
        let listener = TcpListener::bind(LISTEN_ADDRESS).unwrap_or_else(|err| panic!("{:?}", err));
        println!("start proxy with address {}", LISTEN_ADDRESS);
        loop {
            let (mut local_conn, _socket_addr) = listener
                .accept()
                .await
                .expect("Unable to accept connection");

            let _ = monoio::spawn(async move {
                let local_addr = local_conn.local_addr().unwrap();
                let peer_addr = local_conn.peer_addr().unwrap();
                println!("local: {}, peer addr is {}", local_addr, peer_addr);

                let mut remote_conn = TcpStream::connect_addr(peer_addr)
                    .await
                    .expect(&format!("unable to connect addr: {}", peer_addr));

                let (mut local_read, mut local_write) = local_conn.split();
                let (mut remote_read, mut remote_write) = remote_conn.split();
                let _ = monoio::join!(
                    copy_data(&mut local_read, &mut remote_write),
                    copy_data(&mut remote_read, &mut local_write)
                );
                println!("transfer finished");
            });
        }
    });
    fu.await;
}

async fn copy_data<Read: AsyncReadRent, Write: AsyncWriteRent>(
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
