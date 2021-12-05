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
pub async fn main_loop(
    tun: AsyncDevice,
    tunnel: BufReader<SslStream<TcpStream>>,
) -> AsyncReturn<()> {
    let mut tun = tun.into_framed();
    // let tun = BufReader::new(tun);
    let mut tunnel_buf = [0u8; 1480];
    // let mut tun_buf = [0u8; 1480];

    // let (mut tun_reader, mut tun_writer) = tokio::io::split(tun);
    let (mut tunnel_reader, mut tunnel_writer) = tokio::io::split(tunnel);

    loop {
        // let tun_active = tun_reader.read(&mut tun_buf).fuse();
        let tun_active = tun.next().fuse();
        let tunnel_active = tunnel_reader.read(&mut tunnel_buf).fuse();

        pin_mut!(tun_active, tunnel_active);
        select! {
            res  = tun_active => {
                // match res {
                //     Ok(n) => {
                //         if 0 != n {
                //             if 0x45 == tun_buf[0] {
                //                 debug!("Write {:x?}", &tun_buf[..n]);
                //                 tunnel_writer.write_all(&tun_buf[..n]).await.unwrap();
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
                    tunnel_writer.write_all(packet.get_bytes()).await.unwrap();
                }
            },
            res  = tunnel_active => {
                match res {
                    Ok(n) => {
                        if 0 != n {
                            debug!("Recv {:x?}", &tunnel_buf[..n]);
                            // tun_writer.write_all(&tunnel_buf[0..n]).await.unwrap();
                            tun.send(TunPacket::new(tunnel_buf[0..n].to_vec())).await.unwrap();
                        } else {
                            // debug!("{}", n);
                        }
                    }
                    Err(e) => {
                        info!("close connection {:?}", e);
                    }
                }
            },
        }
    }
}
