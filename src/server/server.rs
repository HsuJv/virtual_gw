use std::{pin::Pin, process::Command};

use crate::common::{self, action};
use crate::tunnel::create_tun;
use crate::AsyncReturn;
use crate::{config, server::clientip};
use log::*;
use openssl::ssl::{Ssl, SslAcceptor, SslFiletype, SslMethod, SslVerifyMode};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::{io::BufReader, net::TcpStream};
use tokio_openssl::SslStream;

use super::clientip::release_client_ip;

async fn server_config(client_ip: &str, tun_addr: &str) -> AsyncReturn<Vec<u8>> {
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
    let mut ret_buf = vec![
        action::CONFIG,
        ((ret.len() & 0xff00) >> 16).try_into().unwrap(),
        (ret.len() & 0xff).try_into().unwrap(),
    ];
    ret_buf.extend(ret.as_bytes());
    Ok(ret_buf)
}

async fn handle_client_connect(
    tun_addr: String,
    mut ssl: BufReader<SslStream<TcpStream>>,
) -> AsyncReturn<()> {
    let tun = create_tun(&tun_addr).await.unwrap();
    let mut client_ip = String::new();
    loop {
        let action = ssl.read_u8().await.unwrap();
        match action {
            action::CONFIG => {
                info!("Connection start");
                let _len = ssl.read_u16().await.unwrap();
                let config_magic = ssl.read_u32().await.unwrap();
                if config_magic != action::CONFIG_MAGIC {
                    error!(
                        "Invalid config magic, expect {:x}, got {:x}",
                        action::CONFIG_MAGIC,
                        config_magic
                    );
                    return AsyncReturn::Err("Invalid config magic".into());
                }

                client_ip = clientip::generate_client_ip().unwrap();
                let client_param = server_config(&client_ip, &tun_addr).await?;
                let _ = ssl.write(&client_param).await;
            }
            action::CONNECT => {
                let _len = ssl.read_u16().await.unwrap();
                let connect_magic = ssl.read_u32().await.unwrap();
                if connect_magic != action::CONNECT_MAGIC {
                    error!(
                        "Invalid connect magic, expect {:x}, got {:x}",
                        action::CONNECT_MAGIC,
                        connect_magic
                    );
                    return AsyncReturn::Err("Invalid connect magic".into());
                }
                let _ = ssl.write(&action::CONNECT_BUF).await;
                // Tunnel setup
                break;
            }
            _ => unimplemented!(),
        }
    }
    if client_ip.is_empty() {
        error!("Invalid client ip");
        return AsyncReturn::Err("Client ip is empty".into());
    }
    match common::main_loop(tun, ssl).await {
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
    // Create the TLS acceptor.
    let tls_acceptor = {
        let mut tls_acceptor =
            SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server()).unwrap();

        tls_acceptor.set_verify_callback(SslVerifyMode::PEER, |_, _| true);
        tls_acceptor
            .set_private_key_file(config::get_key_file(), SslFiletype::PEM)
            .unwrap();
        tls_acceptor
            .set_certificate_file(config::get_cert_file(), SslFiletype::PEM)
            .unwrap();
        tls_acceptor.set_ca_file(config::get_ca_file()).unwrap();
        tls_acceptor.build()
    };

    loop {
        let (socket, client) = listener.accept().await?;
        let tun_addr = tun_addr.clone();
        let tls_acceptor = tls_acceptor.clone();
        info!("Accept client {}", client);

        tokio::spawn(async move {
            let ssl = Ssl::new(tls_acceptor.context()).unwrap();
            let mut tls_stream = SslStream::new(ssl, socket).unwrap();
            Pin::new(&mut tls_stream).accept().await.unwrap();
            let client_stream = BufReader::new(tls_stream);
            let _ = handle_client_connect(tun_addr, client_stream).await;
        });
    }
}
