use std::process::Command;

use crate::common::{self, action};
use crate::tunnel::create_tun;
use crate::AsyncReturn;
use crate::{config, server::clientip};
use log::*;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::{io::BufReader, net::TcpStream};

use super::clientip::release_client_ip;

async fn handle_client_connect(tun_addr: String, mut s: BufReader<TcpStream>) -> AsyncReturn<()> {
    let tun = create_tun(&tun_addr).await.unwrap();
    let action = s.read_u8().await.unwrap();
    let mut client_ip = String::new();
    match action {
        action::CONNCET => {
            info!("Connection start");
            let len = s.read_u16().await.unwrap();
            let mut buf = Vec::with_capacity(len as usize);
            for _ in 0..len as usize {
                let byte = s.read_u8().await.unwrap();
                buf.push(byte);
            }

            client_ip = clientip::get_new_client_ip().unwrap();
            info!("route add {} gw {}", client_ip, tun_addr);
            let _ = Command::new("route")
                .arg("add")
                .arg("-host")
                .arg(&client_ip)
                .arg("gw")
                .arg(&tun_addr)
                .output()
                .expect("failed to add routes");

            let ret = json!({
                "ip": &client_ip,
                "routes": config::get_client_routes()
            })
            .to_string();
            debug!("Send {:?}", ret);
            let mut to_write = vec![
                action::CONNCET,
                ((ret.len() & 0xff00) >> 16).try_into().unwrap(),
                (ret.len() & 0xff).try_into().unwrap(),
            ];
            to_write.extend(ret.as_bytes());
            let _ = s.write(&to_write).await;
        }
        _ => unimplemented!(),
    }
    match common::main_loop(tun, s).await {
        Err(e) => info!("Close connection {}", e),
        _ => panic!(),
    };
    info!("Release client ip {}", client_ip);
    release_client_ip(&client_ip);
    Ok(())
}

pub async fn start() -> AsyncReturn<()> {
    clientip::init();

    let net_addr = config::get_listen_ip();
    let tun_addr = config::get_server_ip();
    // let tun = create_tun(&tun_addr).await?;
    let listener = TcpListener::bind(&net_addr).await?;
    info!("Server started at {}", net_addr);

    loop {
        let (socket, client) = listener.accept().await?;
        let tun_addr = tun_addr.clone();
        info!("Accept client {}", client);

        tokio::spawn(async move {
            let client_stream = BufReader::new(socket);
            let _ = handle_client_connect(tun_addr, client_stream).await;
        });
    }
}
