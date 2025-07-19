use log::LevelFilter;
use std::{env, sync::OnceLock};

#[derive(Debug)]
pub struct Config {
    pub listen_ip: String,
    pub port: u16,

    /// default: info
    pub log_level: log::LevelFilter,

    /// default: 512MB
    pub string_max_length: usize,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init_config() -> &'static Config {
    CONFIG.get_or_init(|| Config {
        listen_ip: env::var("RUDIS_LISTEN_IP").unwrap_or("127.0.0.1".into()),
        port: env::var("RUDIS_PORT")
            .map(|p| p.parse().expect("invalid RUDIS_PORT"))
            .unwrap_or(6379),

        log_level: env::var("RUDIS_LOG_LEVEL")
            .map(|level| level.parse().expect("invalid RUDIS_LOG_LEVEL"))
            .unwrap_or(LevelFilter::Info),

        string_max_length: env::var("RUDIS_STRING_MAX_LENGTH")
            .map(|len| len.parse().expect(""))
            .unwrap_or(2 << 25),
    })
}

pub fn get_config() -> &'static Config {
    &CONFIG.get().expect("config hasn't been initialized")
}
