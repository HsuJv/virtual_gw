use std::sync::Mutex;
use std::vec;

use crate::config;

static mut IP_POOL: Option<&Mutex<Vec<String>>> = None;

pub fn init() {
    unsafe {
        if IP_POOL.is_some() {
            panic!("init twice");
        }
    }
    let client_ip = config::get_client_ips();
    let client_ip: Vec<&str> = client_ip.split('/').collect();
    let (net, mask) = (client_ip[0], client_ip[1]);
    let net: Vec<&str> = net.split('.').collect();
    let mask = mask.parse::<u8>().unwrap();
    let dig_net: Vec<u8> = net.iter().map(|x| x.parse::<u8>().unwrap()).collect();
    let dig_net = (dig_net[0] as u32) << 24
        | (dig_net[1] as u32) << 16
        | (dig_net[2] as u32) << 8
        | (dig_net[3] as u32);
    let ip_num = (1 << (32 - mask)) as usize;

    let mut ips = Vec::with_capacity(ip_num - 2);
    for i in 1..ip_num - 1 {
        let client_ip = dig_net + i as u32;
        let client_ip = vec![
            client_ip >> 24,
            (client_ip & 0x00ff0000) >> 16,
            (client_ip & 0x0000ff00) >> 8,
            (client_ip & 0x000000ff),
        ]
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(".");
        ips.push(client_ip);
    }

    unsafe {
        IP_POOL = Some(Box::leak(Box::new(Mutex::new(ips))));
    }
}

pub fn generate_client_ip() -> Option<String> {
    unsafe { IP_POOL.unwrap().lock().unwrap().pop() }
}

pub fn release_client_ip(s: &str) {
    unsafe { IP_POOL.unwrap().lock().unwrap().push(s.to_string()) };
}
