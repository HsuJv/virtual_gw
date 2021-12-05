use std::{pin::Pin, process::Command};

use crate::config;
use crate::tunnel::create_tun;
use crate::AsyncReturn;
use crate::{
    common::{self, action},
    server::ippool::IpPool,
};
use log::*;
use openssl::ssl::{Ssl, SslAcceptor, SslFiletype, SslMethod, SslVerifyMode};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::{io::BufReader, net::TcpStream};
use tokio_openssl::SslStream;

async fn server_config(client_ip: &str, server_ip: &str) -> AsyncReturn<Vec<u8>> {
    info!("route add {} gw {}", client_ip, server_ip);
    let _ = Command::new("route")
        .arg("add")
        .arg("-host")
        .arg(&client_ip)
        .arg("gw")
        .arg(&server_ip)
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
    cips: IpPool,
    sips: IpPool,
    mut ssl: BufReader<SslStream<TcpStream>>,
) -> AsyncReturn<()> {
    let client_ip = cips.get_ip().await?;
    let server_ip = sips.get_ip().await?;
    let tun = create_tun(&server_ip).await.unwrap();
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

                let client_param = server_config(&client_ip, &server_ip).await?;
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
    if sips.is_host_mode() {
        unimplemented!();
    } else {
        match common::main_loop(tun, ssl).await {
            Err(e) => info!("Close connection {}", e),
            _ => panic!(),
        };
        cips.free_ip(&client_ip).await?;
        sips.free_ip(&server_ip).await?;
    }
    Ok(())
}

pub async fn start() -> AsyncReturn<()> {
    let server_ips = IpPool::new("Server_ips", &config::get_server_ip());
    let client_ips = IpPool::new("Client_ips", &config::get_client_ip());

    let listen_addr = config::get_listen_ip();
    let listener = TcpListener::bind(&listen_addr).await?;
    info!("Server started at {} with {} mode", listen_addr, {
        if server_ips.is_host_mode() {
            "single tun"
        } else {
            "multiple tun"
        }
    });
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
        let tls_acceptor = tls_acceptor.clone();
        info!("Accept client {}", client);
        let client_ips = client_ips.clone();
        let server_ips = server_ips.clone();

        tokio::spawn(async move {
            let ssl = Ssl::new(tls_acceptor.context()).unwrap();
            let mut tls_stream = SslStream::new(ssl, socket).unwrap();
            Pin::new(&mut tls_stream).accept().await.unwrap();
            let client_stream = BufReader::new(tls_stream);
            let _ = handle_client_connect(client_ips, server_ips, client_stream).await;
        });
    }
}
