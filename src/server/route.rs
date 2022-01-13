use etherparse::{Ipv4HeaderSlice, Ipv6HeaderSlice};
use futures::{future::FutureExt, pin_mut, select};
use futures::{SinkExt, StreamExt};
use log::*;
use std::net::IpAddr;
use std::{collections::HashMap, sync::RwLock};
use tokio::sync::mpsc;
use tun::{AsyncDevice, TunPacket};

use crate::AsyncReturn;

pub enum RouteMsg {
    AddRoute(String, mpsc::Sender<TunPacket>),
    DelRoute(String),
    Forwarding(TunPacket),
}

struct RouterInner {
    e: Option<AsyncDevice>,
    t: RwLock<HashMap<IpAddr, mpsc::Sender<TunPacket>>>,
}

impl RouterInner {
    pub fn new(e: AsyncDevice) -> RouterInner {
        RouterInner {
            e: Some(e),
            t: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, ip: &str, session_addr: mpsc::Sender<TunPacket>) {
        let mut t = self.t.write().unwrap();
        let ip = ip.parse::<IpAddr>().unwrap();

        t.insert(ip, session_addr);
    }

    pub fn del(&self, ip: &str) {
        let ip = ip.parse::<IpAddr>().unwrap();
        let mut t = self.t.write().unwrap();
        t.remove(&ip);
    }

    pub async fn routing(&self, pkt: TunPacket) {
        let version = pkt.get_bytes()[0] >> 4;

        if let Some(ip) = match version {
            4 => {
                let ip4h = Ipv4HeaderSlice::from_slice(pkt.get_bytes()).unwrap();
                Some(ip4h.destination_addr().into())
            }
            6 => {
                let ip6h = Ipv6HeaderSlice::from_slice(pkt.get_bytes()).unwrap();
                Some(ip6h.destination_addr().into())
            }
            x => {
                debug!("Unimplement packet version {}", x);
                None
            }
        } {
            let session_addr = {
                let route_table = self.t.read().unwrap();
                route_table.get(&ip).cloned()
            };
            if let Some(session_addr) = session_addr {
                session_addr.send(pkt).await.unwrap();
            } else {
                warn!("No session for ip {:x?}", ip);
            }
        }
    }

    pub async fn start(mut self) -> AsyncReturn<mpsc::Sender<RouteMsg>> {
        if self.e.is_none() {
            panic!("No underlay device");
        }
        let (msg_addr, mut msg_rcv) = mpsc::channel::<RouteMsg>(100);

        let tun = self.e.take().unwrap();
        tokio::spawn(async move {
            let mut tun = tun.into_framed();
            loop {
                let tun_input = tun.next().fuse();
                let route_msg = msg_rcv.recv().fuse();
                pin_mut!(tun_input, route_msg);
                select! {
                    res  = tun_input => {
                        if let Ok(packet) = res.unwrap() {
                            // debug!("Read {:#04x?} from tun", packet.get_bytes().len());
                            self.routing(packet).await;
                        }
                    },

                    res = route_msg => {
                        if let Some(msg) = res {
                            match msg {
                                RouteMsg::Forwarding(pkt) => {
                                    debug!("Write {:#04x?} to tun", pkt.get_bytes().len());
                                    let _ = tun.send(pkt).await;
                                }
                                RouteMsg::AddRoute(ip, session_addr) => {
                                    debug!("Add ip {} to routing", ip);
                                    self.add(&ip, session_addr);
                                }
                                RouteMsg::DelRoute(ip) => {
                                    debug!("Del ip {} from routing", ip);
                                    self.del(&ip);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(msg_addr)
    }
}

pub struct Router(RouterInner);

impl Router {
    pub fn new(endpoint: AsyncDevice) -> Router {
        Router(RouterInner::new(endpoint))
    }

    pub async fn start(self) -> AsyncReturn<mpsc::Sender<RouteMsg>> {
        self.0.start().await
    }
}
