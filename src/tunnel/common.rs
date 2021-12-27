use crate::AsyncReturn;
use futures::{future::FutureExt, pin_mut, select};
use futures::{SinkExt, StreamExt};
use log::*;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tokio_openssl::SslStream;
use tun::{AsyncDevice, TunPacket};

pub async fn main_loop(tun: AsyncDevice, ssl: BufReader<SslStream<TcpStream>>) -> AsyncReturn<()> {
    let mut tun = tun.into_framed();
    let mut ssl_buf = [0u8; 1600];

    let (mut ssl_reader, mut ssl_writer) = tokio::io::split(ssl);

    loop {
        let tun_active = tun.next().fuse();
        let ssl_active = ssl_reader.read(&mut ssl_buf).fuse();

        pin_mut!(tun_active, ssl_active);
        select! {
            res  = tun_active => {
                if let Ok(packet) = res.unwrap() {
                    debug!("Write {:02x?}", packet.get_bytes().len());
                    ssl_writer.write_all(packet.get_bytes()).await.unwrap();
                }
            },
            res  = ssl_active => {
                let n = res.unwrap();
                if 0 != n {
                    debug!("Recv {:02x?}", &ssl_buf.len());
                    tun.send(TunPacket::new(ssl_buf.to_vec())).await.unwrap();
                } else {
                    return Ok(());
                }
            },
        }
    }
}
