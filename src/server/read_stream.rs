use log::{debug, info};
use std::io::Result;
use std::net::Shutdown;
use tokio::io::AsyncReadExt;
use tokio::io::ReadHalf;
use tokio::net::TcpStream;

#[derive(Debug)]
pub(super) struct ReadStream {
    stream: ReadHalf<TcpStream>,
    ssl_required: bool,
}

impl ReadStream {
    pub(super) fn new(stream: ReadHalf<TcpStream>) -> Self {
        Self {
            stream,
            ssl_required: false,
        }
    }

    pub(super) async fn read(&mut self, buff: &mut [u8]) -> Result<usize> {
        if self.ssl_required {}
        self.stream.read(buff).await
    }

    pub(super) fn set_ssl(&mut self, ssl_required: bool) {
        self.ssl_required = ssl_required;
    }
}
