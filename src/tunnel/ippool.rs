use crate::AsyncReturn;
use log::*;
use std::sync::{Arc, Mutex};

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

static mut POOL: Option<&IpPool> = None;

pub struct IpPool(Arc<IpPoolInner>);

pub fn init(name: &str, ips: &str) -> AsyncReturn<()> {
    unsafe {
        if POOL.is_some() {
            panic!("Cannot init twice");
        }
    }
    let pool = Box::new(IpPool::new(name, ips)?);
    unsafe {
        POOL = Some(Box::leak(pool));
    }
    Ok(())
}

pub fn generate_client_ip() -> AsyncReturn<String> {
    unsafe {
        if POOL.is_none() {
            panic!("Cannot generate ip before init");
        }
    }
    unsafe { POOL.unwrap().get_ip() }
}

pub fn release_client_ip(ip: &str) -> AsyncReturn<()> {
    unsafe {
        if POOL.is_none() {
            panic!("Cannot release ip before init");
        }
    }
    unsafe { POOL.unwrap().free_ip(ip) }
}

impl IpPool {
    pub fn new(name: &str, ips: &str) -> AsyncReturn<IpPool> {
        let ips: Vec<&str> = ips.split('/').collect();
        if ips.len() != 2 {
            let ips = ips.iter().map(|x| x.to_string()).collect::<Vec<String>>();
            Ok(IpPool(Arc::new(IpPoolInner {
                name: name.to_string(),
                ips: Mutex::new(ips),
                mode: IpPooMode::Host,
            })))
        } else {
            let (net, mask) = (ips[0], ips[1]);
            let net: Vec<&str> = net.split('.').collect();
            let mask = mask.parse::<u8>().unwrap();
            if mask > 30 {
                return Err("Network mask cannot be larger than 30".into());
            }
            let dig_net: Vec<u8> = net.iter().map(|x| x.parse::<u8>().unwrap()).collect();
            let dig_net = u32::from_be_bytes(dig_net.try_into().unwrap());
            let ip_num = (1 << (32 - mask)) as usize;

            let mut ips = Vec::with_capacity(ip_num - 2);
            for i in 1..ip_num - 1 {
                let ipe = dig_net + i as u32;
                let ipe = ipe
                    .to_be_bytes()
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(".");
                ips.push(ipe);
            }
            Ok(IpPool(Arc::new(IpPoolInner {
                name: name.to_string(),
                ips: Mutex::new(ips),
                mode: IpPooMode::Network,
            })))
        }
    }

    pub fn test_new(name: &str, ips: &str) -> AsyncReturn<IpPool> {
        let ips: Vec<&str> = ips.split('/').collect();
        if ips.len() != 2 {
            if ips.len() != 1 {
                return Err("Not a valid network address".into());
            }

            Ok(IpPool(Arc::new(IpPoolInner {
                name: name.to_string(),
                ips: Mutex::new(vec![]),
                mode: IpPooMode::Host,
            })))
        } else {
            let (net, mask) = (ips[0], ips[1]);
            let net: Vec<&str> = net.split('.').collect();
            let mask = mask.parse::<u8>().unwrap();
            if mask > 30 {
                return Err("Network mask cannot be larger than 30".into());
            }
            let dig_net: Vec<u8> = net.iter().map(|x| x.parse::<u8>().unwrap()).collect();
            let _dig_net = u32::from_be_bytes(dig_net.try_into().unwrap());
            let _ip_num = (1 << (32 - mask)) as usize;
            Ok(IpPool(Arc::new(IpPoolInner {
                name: name.to_string(),
                ips: Mutex::new(vec![]),
                mode: IpPooMode::Network,
            })))
        }
    }

    pub fn get_ip(&self) -> AsyncReturn<String> {
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

    pub fn free_ip(&self, ip: &str) -> AsyncReturn<()> {
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

    #[allow(dead_code)]
    pub fn is_host_mode(&self) -> bool {
        self.0.mode == IpPooMode::Host
    }
}

impl Clone for IpPool {
    fn clone(&self) -> IpPool {
        IpPool(Arc::clone(&self.0))
    }
}
