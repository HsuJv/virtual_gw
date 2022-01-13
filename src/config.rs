use config::{Config, ConfigError, File, Value};

use crate::tunnel::ippool::IpPool;
use paste::paste;
use std::{collections::HashMap, net::SocketAddr, path::Path};

struct PrivConfig {
    conf: Config,
}

#[allow(dead_code)]
impl PrivConfig {
    fn get_str(&self, key: &str) -> Result<String, ConfigError> {
        self.conf.get_str(key)
    }

    fn get_int(&self, key: &str) -> Result<i64, ConfigError> {
        self.conf.get_int(key)
    }

    fn get_float(&self, key: &str) -> Result<f64, ConfigError> {
        self.conf.get_float(key)
    }

    fn get_bool(&self, key: &str) -> Result<bool, ConfigError> {
        self.conf.get_bool(key)
    }

    fn get_table(&self, key: &str) -> Result<HashMap<String, Value>, ConfigError> {
        self.conf.get_table(key)
    }

    fn get_array(&self, key: &str) -> Result<Vec<Value>, ConfigError> {
        self.conf.get_array(key)
    }
}

static mut CONFIG: Option<&PrivConfig> = None;

fn config_check() {
    if is_server() {
        let test_server_ip = IpPool::test_new("test", &get_server_ip_panic()).unwrap();
        assert!(test_server_ip.is_host_mode());

        let _test_listen_ip = get_listen_ip_panic().parse::<SocketAddr>().unwrap();

        let _test_client_ip = IpPool::test_new("test", &get_client_ip_panic()).unwrap();

        for test_route in get_client_routes_panic() {
            let _test_pool = IpPool::test_new("test", &test_route).unwrap();
        }
    } else {
        let _test_server_ip = get_server_ip_panic().parse::<SocketAddr>().unwrap();
    }

    let ca_file_path = get_ca_file_panic();
    if !Path::new(&ca_file_path).exists() {
        panic!(
            "Cannot find certification authority file @ path {}",
            ca_file_path
        );
    }

    let cert_file_path = get_cert_file_panic();
    if !Path::new(&cert_file_path).exists() {
        panic!("Cannot find certification file @ path {}", cert_file_path);
    }

    let key_file_path = get_key_file_panic();
    if !Path::new(&key_file_path).exists() {
        panic!("Cannot find the key file @ path {}", key_file_path);
    }
}

pub fn init_from_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if CONFIG.is_some() {
            panic!("Cannot init twice");
        }
    }

    let mut conf = Config::default();
    conf.merge(File::with_name(filename))?;
    let priv_config = Box::new(PrivConfig { conf });
    unsafe {
        CONFIG = Some(Box::leak(priv_config));
    }

    config_check();
    Ok(())
}

pub fn is_server() -> bool {
    unsafe { CONFIG.unwrap().get_bool("server").unwrap_or(false) }
}

macro_rules! impl_getter {
    (_ String, $field:ident) => {
        unsafe { CONFIG.unwrap().get_str(stringify!($field)).unwrap() }
    };

    (_ Vec<String>, $field:ident) => {
        unsafe {
            CONFIG
                .unwrap()
                .get_array(stringify!($field))
                .unwrap()
                .iter()
                .map(|i| i.clone().into_str().unwrap())
                .collect()
        }
    };

    (_ String, $field:ident, $default: expr) => {
        unsafe {
            CONFIG
                .unwrap()
                .get_str(stringify!($field))
                .unwrap_or($default)
        }
    };

    (_ Vec<String>, $field:ident, $default: expr) => {
        unsafe {
            CONFIG
                .unwrap()
                .get_array(stringify!($field))
                .unwrap_or($default)
                .iter()
                .map(|i| i.clone().into_str().unwrap())
                .collect()
        }
    };

    ($ret:ty, $field:ident, $default: expr) => {
        paste! {
            pub fn [<get_ $field>]() -> $ret {
                impl_getter!(_ $ret, $field, $default)
            }

            fn [<get_ $field _panic>]() -> $ret {
                impl_getter!(_ $ret, $field)
            }
        }
    };
}

impl_getter!(String, listen_ip, "0.0.0.0:443".to_string());
impl_getter!(String, server_ip, "173.75.2.1".to_string());
impl_getter!(String, client_ip, "173.75.1.0/24".to_string());
impl_getter!(Vec<String>, client_routes, vec![]);
impl_getter!(String, ca_file, "ca.cer".to_string());
impl_getter!(String, key_file, "key.pem".to_string());
impl_getter!(String, cert_file, "cert.pem".to_string());
