use serde_derive::Serialize;
use serde_json::{error::Result, from_str};
use std::default::Default;

// const PING: &str = "PING\r\n";
// const PONG: &str = "PONG\r\n";
// const OK: &str = "+OK\r\n";

#[derive(Debug, Serialize)]
pub(super) struct Info {
    server_id: String,
    version: String,
    host: String,
    port: u16,
    auth_required: bool,
    ssl_required: bool,
    max_payload: usize,
}

impl Info {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn set_server_id(mut self, server_id: String) -> Self {
        self.server_id = server_id;
        self
    }

    pub(super) fn set_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    pub(super) fn set_host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub(super) fn set_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub(super) fn set_auth_required(mut self, auth_required: bool) -> Self {
        self.auth_required = auth_required;
        self
    }

    pub(super) fn set_ssl_required(mut self, ssl_required: bool) -> Self {
        self.ssl_required = ssl_required;
        self
    }

    pub(super) fn set_max_payload(mut self, max_payload: usize) -> Self {
        self.max_payload = max_payload;
        self
    }

    pub(super) fn format(&self) -> Result<String> {
        #[cfg(target = "windows")]
        return Ok(format!("INFO {}\r\n", serde_json::to_string(self)?));

        #[cfg(not(target = "windows"))]
        return Ok(format!("INFO {}\n", serde_json::to_string(self)?));
    }
}

impl Default for Info {
    fn default() -> Self {
        Self {
            server_id: String::new(),
            version: String::new(),
            host: "127.0.0.1".to_string(),
            port: 8090,
            auth_required: false,
            ssl_required: false,
            max_payload: 512,
        }
    }
}

#[derive(Debug)]
pub(super) struct Ping;

impl Ping {
    pub(super) const fn format() -> &'static str {
        #[cfg(target = "windows")]
        return "PING\r\n";

        #[cfg(not(target = "windows"))]
        return "PING\n";
    }
}

#[derive(Debug)]
pub(super) struct Pong;

impl Pong {
    pub(super) const fn format() -> &'static str {
        #[cfg(target = "windows")]
        return "PONG\r\n";

        #[cfg(not(target = "windows"))]
        "PONG\n"
    }
}

#[derive(Debug)]
pub(super) struct ResponseOk;

impl ResponseOk {
    pub(super) const fn format() -> &'static str {
        #[cfg(target = "windows")]
        return "+OK\r\n";

        #[cfg(not(target = "windows"))]
        return "+OK\n";
    }
}
