use crate::AsyncReturn;
use futures::{future::FutureExt, pin_mut, select};
use log::*;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tokio_openssl::SslStream;
// use tokio_tun::Tun;
use futures::{SinkExt, StreamExt};
use tun::{AsyncDevice, TunPacket};

// pub async fn main_loop(tun: Tun, tunnel: BufReader<SslStream<TcpStream>>) -> AsyncReturn<()> {
pub async fn main_loop(tun: AsyncDevice, ssl: BufReader<SslStream<TcpStream>>) -> AsyncReturn<()> {
    let mut tun = tun.into_framed();
    // let tun = BufReader::new(tun);
    let mut ssl_buf = [0u8; 1480];
    // let mut tun_buf = [0u8; 1480];

    // let (mut tun_reader, mut tun_writer) = tokio::io::split(tun);
    let (mut ssl_reader, mut ssl_writer) = tokio::io::split(ssl);

    loop {
        // let tun_active = tun_reader.read(&mut tun_buf).fuse();
        let tun_active = tun.next().fuse();
        let ssl_active = ssl_reader.read(&mut ssl_buf).fuse();

        pin_mut!(tun_active, ssl_active);
        select! {
            res  = tun_active => {
                // match res {
                //     Ok(n) => {
                //         if 0 != n {
                //             if 0x45 == tun_buf[0] {
                //                 debug!("Write {:x?}", &tun_buf[..n]);
                //                 ssl_writer.write_all(&tun_buf[..n]).await.unwrap();
                //             } else {
                //                 // debug!("{:?}", n);
                //             }
                //         }
                //     }
                //     Err(e) => {
                //         info!("close connection {:?}", e);
                //     }
                // }
                if let Ok(packet) = res.unwrap() {
                    debug!("Write {:02x?}", packet.get_bytes());
                    ssl_writer.write_all(packet.get_bytes()).await.unwrap();
                }
            },
            res  = ssl_active => {
                let n = res.unwrap();
                if 0 != n {
                    debug!("Recv {:02x?}", &ssl_buf[..n]);
                    // tun_writer.write_all(&ssl_buf[0..n]).await.unwrap();
                    tun.send(TunPacket::new(ssl_buf[0..n].to_vec())).await.unwrap();
                } else {
                    return Ok(());
                }
            },
        }
    }
}
