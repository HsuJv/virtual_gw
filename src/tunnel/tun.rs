use crate::AsyncReturn;
use log::*;
use tun::AsyncDevice;

async fn create_tun_with_ip(ip: &str) -> AsyncReturn<AsyncDevice> {
    let mut config = tun::Configuration::default();

    config.address(ip).netmask((255, 255, 255, 255)).up();

    #[cfg(target_os = "linux")]
    config.platform(|config| {
        config.packet_information(true);
    });

    Ok(tun::create_as_async(&config).unwrap())
    // use futures::StreamExt;
    // use packet::ip::Packet;
    // let mut stream = dev.into_framed();

    // while let Some(packet) = stream.next().await {
    //     match packet {
    //         Ok(pkt) => println!("pkt: {:#?}", Packet::unchecked(pkt.get_bytes())),
    //         Err(err) => panic!("Error: {:?}", err),
    //     }
    // }
}

pub async fn create_tun(addr: &str) -> AsyncReturn<AsyncDevice> {
    let dev = create_tun_with_ip(addr).await?;
    info!("Crate tun : {}", addr);
    Ok(dev)
}
