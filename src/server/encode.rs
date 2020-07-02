use serde_derive::Serialize;
use serde_json::error::Result;
use std::default::Default;
use std::net::IpAddr;

const PING: &[u8; 6] = b"PING\r\n";
const PONG: &[u8; 6] = b"PONG\r\n";
const OK: &[u8; 5] = b"+OK\r\n";

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
    pub(super) fn format() -> Vec<u8> {
        PING.to_vec()
    }
}

#[derive(Debug)]
pub(super) struct Pong;

impl Pong {
    pub(super) fn format() -> Vec<u8> {
        PONG.to_vec()
    }
}

#[derive(Debug)]
pub(super) struct ResponseOk;

impl ResponseOk {
    pub(super) fn format() -> Vec<u8> {
        OK.to_vec()
    }
}

#[derive(Debug)]
pub(super) struct Msg {
    front_chunk: Vec<u8>,
    after_chunk: Vec<u8>,
}

impl Msg {
    pub(super) fn new<'a>(subject: &'a str, reply_to: Option<&'a str>, content: &'a str) -> Self {
        let content_len_str: String = content.len().to_string();
        let mut front_chunk: Vec<u8> = Vec::with_capacity(4 + subject.as_bytes().len() + 1);

        front_chunk.extend_from_slice(b"MSG ");
        front_chunk.extend_from_slice(subject.as_bytes());
        front_chunk.extend_from_slice(b" ");

        let mut after_chunk: Vec<u8> = Vec::with_capacity(
            {
                match reply_to {
                    Some(reply) => reply.as_bytes().len() + 2,
                    None => 1,
                }
            } + content_len_str.as_bytes().len()
                + b"\r\n".len() * 2
                + content.as_bytes().len(),
        );

        after_chunk.extend_from_slice(b" ");
        if let Some(reply) = reply_to {
            after_chunk.extend_from_slice(reply.as_bytes());
            after_chunk.extend_from_slice(b" ");
        }
        after_chunk.extend_from_slice(content_len_str.as_bytes());
        after_chunk.extend_from_slice(b"\r\n");
        after_chunk.extend_from_slice(content.as_bytes());
        after_chunk.extend_from_slice(b"\r\n");

        Self {
            front_chunk,
            after_chunk,
        }
    }

    pub(super) fn format(&self, sid: &str) -> Vec<u8> {
        let mut response: Vec<u8> = Vec::with_capacity(
            self.front_chunk.len() + sid.as_bytes().len() + self.after_chunk.len(),
        );

        response.extend_from_slice(&self.front_chunk);
        response.extend_from_slice(sid.as_bytes());
        response.extend_from_slice(&self.after_chunk);

        response
    }
}
