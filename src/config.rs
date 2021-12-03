use config::{Config, ConfigError, File, Value};

use paste::paste;
use std::collections::HashMap;

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

fn config_check(_conf: &Config) {}

pub fn init_from_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if CONFIG.is_some() {
            panic!("Cannot init twice");
        }
    }

    let mut conf = Config::default();
    conf.merge(File::with_name(filename))?;

    config_check(&conf);

    let priv_config = Box::new(PrivConfig { conf });
    unsafe {
        CONFIG = Some(Box::leak(priv_config));
    }
    Ok(())
}

pub fn is_server() -> bool {
    unsafe { CONFIG.unwrap().get_bool("server").unwrap_or(false) }
}

// macro_rules! get_config_str {
//     ( $field:tt ) => {
//         unsafe { CONFIG.unwrap().get_str($field).unwrap_or("") }
//     };
// }

macro_rules! impl_getter {
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
        }
    };
}

impl_getter!(String, listen_ip, "0.0.0.0:443".to_string());
impl_getter!(String, server_ip, "173.75.2.1".to_string());
impl_getter!(String, client_ips, "".to_string());
impl_getter!(Vec<String>, client_routes, vec![]);
