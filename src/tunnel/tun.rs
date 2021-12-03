use crate::AsyncReturn;
use log::*;
use tokio_tun::{Tun, TunBuilder};

async fn create_tun_with_ip(ip: &str) -> AsyncReturn<Tun> {
    let tun = TunBuilder::new()
        .name("") // if name is empty, then it is set by kernel.
        .tap(false) // false (default): TUN, true: TAP.
        .mtu(1350)
        .address(ip.parse().unwrap())
        .netmask("255.255.255.255".parse().unwrap())
        .packet_info(false) // false: IFF_NO_PI, default is true.
        .up() // or set it up manually using `sudo ip link set <tun-name> up`.
        .try_build()
        .unwrap(); // or `.try_build_mq(queues)` for multi-queue support.

    Ok(tun)
}

// async fn create_tun_with_ip(ip: &str) -> AsyncReturn<AsyncDevice> {
//     let mut config = tun::Configuration::default();

//     config
//         .address(ip)
//         .netmask((255, 255, 255, 255))
//         .mtu(1350)
//         .up();

//     #[cfg(target_os = "linux")]
//     config.platform(|config| {
//         config.packet_information(false);
//     });

//     Ok(tun::create_as_async(&config).unwrap())
// }

pub async fn create_tun(addr: &str) -> AsyncReturn<Tun> {
    let dev = create_tun_with_ip(addr).await?;
    info!("Crate tun : {}", addr);
    Ok(dev)
}
