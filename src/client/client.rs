use crate::{
    common::{self, action},
    config,
};
use crate::{tunnel::create_tun, AsyncReturn};
use log::*;
use std::process::Command;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

async fn start_connect(s: &mut BufReader<TcpStream>) -> AsyncReturn<serde_json::Value> {
    let buf: [u8; 5] = [action::CONNCET, 0, 2, 3, 4];
    let _ = s.write(&buf).await;
    let action = s.read_u8().await?;
    match action {
        action::CONNCET => {
            let len = s.read_u16().await?;
            let mut json_data: Vec<u8> = Vec::with_capacity(len as usize);

            for _ in 0..len {
                json_data.push(s.read_u8().await?);
            }

            let json_str = String::from_utf8(json_data).unwrap();
            debug!("Get Response {}", json_str);
            return Ok(serde_json::from_str(json_str.as_str()).unwrap());
        }
        _ => unimplemented!(),
    }
}

pub async fn start() -> AsyncReturn<()> {
    info!("Client started");
    let server_addr = config::get_server_ip();

    let connection = TcpStream::connect(server_addr).await?;
    let mut stream = BufReader::new(connection);
    let param = start_connect(&mut stream).await?;
    let ip = param.get("ip").unwrap().as_str().unwrap();
    let routes = param.get("routes").unwrap().as_array().unwrap();

    let tun = create_tun(&ip).await?;

    for route in routes {
        let route = route.as_str().unwrap();
        info!("route add {} gw {}", route, ip);
        let _ = Command::new("route")
            .arg("add")
            .arg("-net")
            .arg(route)
            .arg("gw")
            .arg(ip)
            .output()
            .expect("failed to add routes");
    }
    common::main_loop(tun, stream).await
}
