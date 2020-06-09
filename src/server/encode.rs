use serde_derive::Serialize;
use serde_json::error::Result;
use std::default::Default;
use std::net::IpAddr;

const PING: &str = "PING\r\n";
const PONG: &str = "PONG\r\n";
const OK: &str = "+OK\r\n";

#[derive(Debug, Serialize)]
pub(super) struct Info {
    server_id: String,
    server_name: String,
    version: String,
    host: String,
    port: u16,
    auth_required: bool,
    ssl_required: bool,
    max_payload: usize,
    proto: usize,
    client_id: usize,
    client_ip: String,
    git_commit: String,
    go: String,
}

impl Info {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn set_server_id(mut self, server_id: String) -> Self {
        self.server_id = server_id;
        self
    }

    pub(super) fn set_server_name(mut self, server_name: String) -> Self {
        self.server_name = server_name;
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

    pub(super) fn set_proto(mut self, proto: usize) -> Self {
        self.proto = proto;
        self
    }

    pub(super) fn set_client_id(mut self, client_id: usize) -> Self {
        self.client_id = client_id;
        self
    }

    pub(super) fn set_client_ip(mut self, client_ip: IpAddr) -> Self {
        self.client_ip = client_ip.to_string();
        self
    }

    pub(super) fn format(&self) -> Result<String> {
        Ok(format!("INFO {}\r\n", serde_json::to_string(self)?))
    }
}

impl Default for Info {
    fn default() -> Self {
        Self {
            server_id: String::new(),
            server_name: String::new(),
            version: String::new(),
            host: "127.0.0.1".to_string(),
            port: 8090,
            auth_required: false,
            ssl_required: false,
            max_payload: 512,
            proto: 1,
            client_id: 0,
            client_ip: "127.0.0.1".to_string(),
            git_commit: "8c8d6f".to_string(),
            go: "go1.13".to_string(),
        }
    }
}

#[derive(Debug)]
pub(super) struct Ping;

impl Ping {
    pub(super) const fn format() -> &'static str {
        PING
    }
}

#[derive(Debug)]
pub(super) struct Pong;

impl Pong {
    pub(super) const fn format() -> &'static str {
        PONG
    }
}

#[derive(Debug)]
pub(super) struct ResponseOk;

impl ResponseOk {
    pub(super) const fn format() -> &'static str {
        OK
    }
}

#[derive(Debug)]
pub(super) struct Msg<'a> {
    subject: &'a str,
    sid: &'a str,
    reply_to: Option<&'a str>,
    content: &'a str,
}

impl<'a> Msg<'a> {
    pub(super) fn new(
        subject: &'a str,
        sid: &'a str,
        reply_to: Option<&'a str>,
        content: &'a str,
    ) -> Self {
        Self {
            subject,
            sid,
            reply_to,
            content,
        }
    }

    pub(super) fn format(&self) -> &'static str {
        match self.reply_to {
            Some(reply_to) => format!(
                "MSG {} {} {} {}\r\n{}\r\n",
                self.subject,
                self.sid,
                reply_to,
                self.content.len(),
                self.content
            ),
            None => format!(
                "MSG {} {} {}\r\n{}\r\n",
                self.subject,
                self.sid,
                self.content.len(),
                self.content
            ),
        }
    }
}
