use crate::AsyncReturn;
use log::*;

pub async fn start() -> AsyncReturn<()> {
    info!("Client started");
    Ok(())
}
