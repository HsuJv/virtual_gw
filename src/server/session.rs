use futures::{executor, future::FutureExt, pin_mut, select};
use log::*;
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    process::Command,
    sync::mpsc,
};
use tokio_openssl::SslStream;
use tun::TunPacket;

use crate::{config, tunnel::action, AsyncReturn};

use super::{ippool, route::RouteMsg};

// the main structure of the session
#[allow(dead_code)]
struct SessionInner {
    name: String,
    client_ip: String,
    server_ip: String,
    stream: BufReader<SslStream<TcpStream>>,
    router: mpsc::Sender<RouteMsg>,
}

impl SessionInner {
    pub async fn start(mut self) -> AsyncReturn<()> {
        let (addr, tun) = mpsc::channel(100);

        let _ = self
            .router
            .send(RouteMsg::AddRoute(self.client_ip.clone(), addr))
            .await;

        self.handle_params().await?;
        self.main_loop(tun).await
    }
}

impl SessionInner {
    async fn server_config(&self) -> AsyncReturn<Vec<u8>> {
        info!("route add {} gw {}", self.client_ip, self.server_ip);
        let _ = Command::new("route")
            .arg("add")
            .arg("-host")
            .arg(&self.client_ip)
            .arg("gw")
            .arg(&self.server_ip)
            .output()
            .await
            .expect("failed to add routes");

        let ret = json!({
            "ip": &self.client_ip,
            "routes": config::get_client_routes(),
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

    async fn handle_params(&mut self) -> AsyncReturn<()> {
        let client_param = self.server_config().await?;
        loop {
            let action = self.stream.read_u8().await.unwrap();
            match action {
                action::CONFIG => {
                    info!("Connection start");
                    let _len = self.stream.read_u16().await.unwrap();
                    let config_magic = self.stream.read_u32().await.unwrap();
                    if config_magic != action::CONFIG_MAGIC {
                        error!(
                            "Invalid config magic, expect {:x}, got {:x}",
                            action::CONFIG_MAGIC,
                            config_magic
                        );
                        return AsyncReturn::Err("Invalid config magic".into());
                    }

                    let _ = self.stream.write(&client_param).await;
                }
                action::CONNECT => {
                    let _len = self.stream.read_u16().await.unwrap();
                    let connect_magic = self.stream.read_u32().await.unwrap();
                    if connect_magic != action::CONNECT_MAGIC {
                        error!(
                            "Invalid connect magic, expect {:x}, got {:x}",
                            action::CONNECT_MAGIC,
                            connect_magic
                        );
                        return AsyncReturn::Err("Invalid connect magic".into());
                    }
                    let _ = self.stream.write(&action::CONNECT_BUF).await;
                    // Tunnel setup
                    break;
                }
                _ => unimplemented!(),
            }
        }
        Ok(())
    }

    async fn main_loop(&mut self, mut tun: mpsc::Receiver<TunPacket>) -> AsyncReturn<()> {
        let mut ssl_buf = [0u8; 1600];

        loop {
            let ssl_rx = self.stream.read(&mut ssl_buf).fuse();
            let ssl_tx = tun.recv().fuse();

            pin_mut!(ssl_rx, ssl_tx);
            select! {
                res  = ssl_rx => {
                    let n = res.unwrap();
                    if 0 == n {
                        break;
                    } else {
                        debug!("Recv {:#04x?} from client", n);
                        let _ = self.router
                            .send(RouteMsg::Forwarding(TunPacket::new(ssl_buf[..n].to_vec())))
                            .await;
                    }
                },

                res = ssl_tx => {
                    if let Some(pkt) = res {
                        debug!("Write {:#04x?} to client", pkt.get_bytes().len());
                        let _ = self.stream.write_all(pkt.get_bytes()).await;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for SessionInner {
    fn drop(&mut self) {
        info!("Session {}({}) ends", self.name, self.client_ip);
        ippool::release_client_ip(&self.client_ip).unwrap();
        let _ = executor::block_on(self.router.send(RouteMsg::DelRoute(self.client_ip.clone())));
    }
}

pub struct Session(SessionInner);

impl Session {
    pub async fn start(self) -> AsyncReturn<()> {
        self.0.start().await
    }
}

pub struct SessionBuilder {
    name: String,
    server_ip: String,
    stream: Option<BufReader<SslStream<TcpStream>>>,
    router: Option<mpsc::Sender<RouteMsg>>,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        SessionBuilder {
            name: "".to_string(),
            server_ip: "".to_string(),
            stream: None,
            router: None,
        }
    }
}

impl SessionBuilder {
    pub fn new() -> Self {
        SessionBuilder::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn server_ip(mut self, ip: &str) -> Self {
        self.server_ip = ip.to_string();
        self
    }

    pub fn stream(mut self, stream: BufReader<SslStream<TcpStream>>) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn router(mut self, router: mpsc::Sender<RouteMsg>) -> Self {
        self.router = Some(router);
        self
    }

    pub fn build(self) -> Session {
        let name = self.name;
        let client_ip = ippool::generate_client_ip().unwrap();
        let server_ip = self.server_ip;
        let stream = self.stream.unwrap_or_else(|| panic!("No stream"));
        let router = self.router.unwrap_or_else(|| panic!("No router"));

        info!("Client session \"{}\" start", name);
        Session(SessionInner {
            name,
            client_ip,
            server_ip,
            stream,
            router,
        })
    }
}
