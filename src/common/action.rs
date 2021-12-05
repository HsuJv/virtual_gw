pub const CONFIG: u8 = 1;
pub const CONNECT: u8 = 2;
pub const CONFIG_MAGIC: u32 = 0x53435241;
pub const CONNECT_MAGIC: u32 = 0x53434E43;

pub const CONFIG_BUF: [u8; 7] = [
    CONFIG,
    0,
    4,
    (CONFIG_MAGIC >> 24) as u8,
    ((CONFIG_MAGIC & 0x00ff0000) >> 16) as u8,
    ((CONFIG_MAGIC & 0x0000ff00) >> 8) as u8,
    (CONFIG_MAGIC & 0x000000ff) as u8,
];
pub const CONNECT_BUF: [u8; 7] = [
    CONNECT,
    0,
    4,
    (CONNECT_MAGIC >> 24) as u8,
    ((CONNECT_MAGIC & 0x00ff0000) >> 16) as u8,
    ((CONNECT_MAGIC & 0x0000ff00) >> 8) as u8,
    (CONNECT_MAGIC & 0x000000ff) as u8,
];
