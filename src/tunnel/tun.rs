use crate::AsyncReturn;
use log::*;
use tun::AsyncDevice;

async fn create_tun_with_ip(ip: &str) -> AsyncReturn<AsyncDevice> {
    let mut config = tun::Configuration::default();

    config
        .address(ip)
        .netmask((255, 255, 255, 255))
        .mtu(1350)
        .up();

    #[cfg(target_os = "linux")]
    config.platform(|config| {
        config.packet_information(false);
    });

    Ok(tun::create_as_async(&config).unwrap())
}

pub async fn create_tun(addr: &str) -> AsyncReturn<AsyncDevice> {
    let dev = create_tun_with_ip(addr).await?;
    info!("Crate tun : {}", addr);
    Ok(dev)
}
