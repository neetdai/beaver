use super::config::Config;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref CONFIG: Config = match Config::new("Config.toml") {
        Ok(config) => config,
        Err(e) => {
            panic!("{:?}", e);
        }
    };
}
