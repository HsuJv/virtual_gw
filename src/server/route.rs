use etherparse::Ipv4HeaderSlice;
use log::*;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;
use tun::TunPacket;

struct RouterInner {
    t: RwLock<HashMap<u32, mpsc::Sender<TunPacket>>>,
}

impl RouterInner {
    fn str_ip_to_u32(ip: &str) -> u32 {
        u32::from_be_bytes(
            ip.split('.')
                .collect::<Vec<&str>>()
                .iter()
                .map(|x| x.parse::<u8>().unwrap())
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap(),
        )
    }

    pub fn new() -> RouterInner {
        RouterInner {
            t: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, ip: &str, handle: mpsc::Sender<TunPacket>) {
        let mut t = self.t.write().unwrap();
        let ip = Self::str_ip_to_u32(ip);

        t.insert(ip, handle);
    }

    pub fn del(&self, ip: &str) {
        let ip = Self::str_ip_to_u32(ip);
        let mut t = self.t.write().unwrap();
        t.remove(&ip);
    }

    pub async fn consume(&self, pkt: TunPacket) {
        let version = pkt.get_bytes()[0] >> 4;
        match version {
            4 => {
                let ip4h = Ipv4HeaderSlice::from_slice(pkt.get_bytes()).unwrap();
                let ip = u32::from_be_bytes(ip4h.destination());
                let session_handle = {
                    let route_table = self.t.read().unwrap();
                    let session_handle = route_table.get(&ip);
                    if let Some(session_handle) = session_handle {
                        Some(session_handle.clone())
                    } else {
                        warn!("No session for ip {:x}", ip);
                        None
                    }
                };
                if let Some(session_handle) = session_handle {
                    let session_handle = session_handle.clone();
                    session_handle.send(pkt).await.unwrap();
                }
            }
            x => debug!("Unimplement packet version {}", x),
        }
    }
}

pub struct Router(Arc<RouterInner>);

impl Router {
    pub fn new() -> Router {
        Router(Arc::new(RouterInner::new()))
    }

    pub fn add(&self, ip: &str, handle: mpsc::Sender<TunPacket>) {
        self.0.add(ip, handle);
    }

    pub fn del(&self, ip: &str) {
        self.0.del(ip);
    }

    pub async fn consume(&self, pkt: TunPacket) {
        self.0.consume(pkt).await;
    }
}

impl Clone for Router {
    fn clone(&self) -> Router {
        Router(self.0.clone())
    }
}
