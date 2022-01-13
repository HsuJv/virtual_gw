mod ippool;
mod route;
mod session;

use crate::tunnel::create_tun;
use crate::AsyncReturn;
use crate::{config, server::session::SessionBuilder};
use log::*;
use openssl::nid::Nid;
use openssl::ssl::{Ssl, SslAcceptor, SslFiletype, SslMethod, SslVerifyMode};
use route::Router;
use std::pin::Pin;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio_openssl::SslStream;

pub async fn start() -> AsyncReturn<()> {
    let _ = ippool::init("client IP pool", &config::get_client_ip()).unwrap();
    let router = Router::new(create_tun(&config::get_server_ip()).unwrap());

    let listen_addr = config::get_listen_ip();
    let listener = TcpListener::bind(&listen_addr).await?;

    // Create the TLS acceptor.
    let tls_acceptor = {
        let mut tls_acceptor =
            SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server()).unwrap();

        tls_acceptor
            .set_private_key_file(config::get_key_file(), SslFiletype::PEM)
            .unwrap();
        tls_acceptor
            .set_certificate_file(config::get_cert_file(), SslFiletype::PEM)
            .unwrap();
        tls_acceptor.set_ca_file(config::get_ca_file()).unwrap();
        tls_acceptor.set_verify(SslVerifyMode::PEER);
        tls_acceptor.build()
    };

    // Start tun loop
    let router = router.start().await?;

    // Start server loop
    loop {
        let (socket, client) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let router = router.clone();
        info!("Accept client {}", client);

        tokio::spawn(async move {
            // ssl accept
            let ssl = Ssl::new(tls_acceptor.context()).unwrap();
            let mut tls_stream = SslStream::new(ssl, socket).unwrap();
            Pin::new(&mut tls_stream).accept().await.unwrap();

            // retrieve the common name
            let client_cert = tls_stream.ssl().peer_certificate().unwrap();
            let x509_name = client_cert.subject_name();
            let name = x509_name
                .entries_by_nid(Nid::COMMONNAME)
                .next()
                .unwrap_or_else(|| panic!("No common name found"))
                .data();

            // session build
            let client = SessionBuilder::new()
                .name(&name.as_utf8().unwrap().to_string())
                .server_ip(&config::get_server_ip())
                .stream(BufReader::new(tls_stream))
                .router(router)
                .build();
            let _ = client.start().await;
        });
    }
}
