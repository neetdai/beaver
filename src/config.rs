use serde_derive::Deserialize;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Error as IoError, Read};
use thiserror::Error;
use toml::de::Error as TomlDeserializeError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error `{0}`")]
    Io(#[from] IoError),

    #[error("toml de error `{0}")]
    TomlDeserialize(#[from] TomlDeserializeError),
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    server_id: String,
    server_name: String,
    ip: String,
    port: u16,
    version: String,
    auth_required: bool,
    ssl_required: bool,
    max_payload: usize,
    read_timeout: Option<u64>,
    write_timeout: Option<u64>,
    connect_timeout: Option<u64>,
    proto: usize,
}

impl ServerConfig {
    pub fn get_ip(&self) -> &String {
        &self.ip
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_server_id(&self) -> &String {
        &self.server_id
    }

    pub fn get_server_name(&self) -> &String {
        &self.server_name
    }

    pub fn get_version(&self) -> &String {
        &self.version
    }

    pub fn get_auth_required(&self) -> bool {
        self.auth_required
    }

    pub fn get_ssl_required(&self) -> bool {
        self.ssl_required
    }

    pub fn get_max_payload(&self) -> usize {
        self.max_payload
    }

    pub fn get_proto(&self) -> usize {
        self.proto
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    server: ServerConfig,
}

impl Config {
    pub fn new(config_path: &str) -> Result<Self, Error> {
        let file: File = OpenOptions::new().read(true).open(config_path)?;
        let mut buff: BufReader<File> = BufReader::new(file);
        let mut content: String = String::new();
        buff.read_to_string(&mut content)?;

        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn get_server(&self) -> &ServerConfig {
        &self.server
    }
}
