use crate::AsyncReturn;
use log::*;
use std::sync::{Arc, Mutex};
use std::vec;

#[derive(PartialEq)]
enum IpPooMode {
    Host,
    Network,
}

struct IpPoolInner {
    name: String,
    ips: Mutex<Vec<String>>,
    mode: IpPooMode,
}

pub struct IpPool(Arc<IpPoolInner>);

impl IpPool {
    pub fn new(name: &str, ips: &str) -> IpPool {
        let ips: Vec<&str> = ips.split('/').collect();
        if ips.len() != 2 {
            let ips = ips.iter().map(|x| x.to_string()).collect::<Vec<String>>();
            IpPool(Arc::new(IpPoolInner {
                name: name.to_string(),
                ips: Mutex::new(ips),
                mode: IpPooMode::Host,
            }))
        } else {
            let (net, mask) = (ips[0], ips[1]);
            let net: Vec<&str> = net.split('.').collect();
            let mask = mask.parse::<u8>().unwrap();
            let dig_net: Vec<u8> = net.iter().map(|x| x.parse::<u8>().unwrap()).collect();
            let dig_net = (dig_net[0] as u32) << 24
                | (dig_net[1] as u32) << 16
                | (dig_net[2] as u32) << 8
                | (dig_net[3] as u32);
            // network mask must be less than or equal 30
            // will ensure this while config reading
            let ip_num = (1 << (32 - mask)) as usize;

            let mut ips = Vec::with_capacity(ip_num - 2);
            for i in 1..ip_num - 1 {
                let ipe = dig_net + i as u32;
                let ipe = vec![
                    ipe >> 24,
                    (ipe & 0x00ff0000) >> 16,
                    (ipe & 0x0000ff00) >> 8,
                    (ipe & 0x000000ff),
                ]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(".");
                ips.push(ipe);
            }
            IpPool(Arc::new(IpPoolInner {
                name: name.to_string(),
                ips: Mutex::new(ips),
                mode: IpPooMode::Network,
            }))
        }
    }

    pub async fn get_ip(&self) -> AsyncReturn<String> {
        match self.0.mode {
            IpPooMode::Host => {
                let ips = self.0.ips.lock().unwrap();
                Ok(ips[0].clone())
            }
            IpPooMode::Network => {
                let mut ips = self.0.ips.lock().unwrap();
                if ips.len() == 0 {
                    error!("{} ip pool is empty", self.0.name);
                    Err(format!("Pool {}: No ip left", self.0.name).into())
                } else {
                    let ip = ips.remove(0);
                    debug!("Pool {}: Get ip {}", self.0.name, ip);
                    Ok(ip)
                }
            }
        }
    }

    pub async fn free_ip(&self, ip: &str) -> AsyncReturn<()> {
        match self.0.mode {
            IpPooMode::Host => {
                // do nothing
            }
            IpPooMode::Network => {
                let mut ips = self.0.ips.lock().unwrap();
                debug!("Pool {}: Free ip {}", self.0.name, ip);
                ips.push(ip.to_string());
            }
        }
        Ok(())
    }

    pub fn is_host_mode(&self) -> bool {
        self.0.mode == IpPooMode::Host
    }
}

impl Clone for IpPool {
    fn clone(&self) -> IpPool {
        IpPool(Arc::clone(&self.0))
    }
}
