use log::{debug, error};
use std::io::{Error as IoError, Result as IoResult};
use std::net::SocketAddr;
use std::ops::Drop;
use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::spawn;

#[derive(Debug)]
pub(super) struct WriteStream {
    stream: WriteHalf<TcpStream>,
    ssl_required: bool,

    local_addr: SocketAddr,

    remote_addr: SocketAddr,

    // 是否回复
    verbose: bool,
}

impl WriteStream {
    pub(super) fn new(
        stream: WriteHalf<TcpStream>,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Self {
        Self {
            stream,
            ssl_required: false,
            verbose: true,
            local_addr,
            remote_addr,
        }
    }

    pub(super) async fn write(&mut self, buff: &[u8]) -> IoResult<()> {
        self.stream.write_all(buff).await
    }

    pub(super) async fn send_ok(&mut self, buff: &[u8]) -> IoResult<()> {
        debug!("verbose set {:?}", self.verbose);
        if self.verbose {
            self.write(buff).await?;
        }
        Ok(())
    }

    pub(super) async fn send_err(&mut self, buff: &[u8]) -> IoResult<()> {
        debug!("verbose set {:?}", self.verbose);
        if self.verbose {
            self.stream.write_all(buff).await?;
        }
        Ok(())
    }

    pub(super) async fn shutdown(&mut self) -> IoResult<()> {
        self.stream.shutdown().await
    }

    pub(super) fn set_ssl(&mut self, ssl_required: bool) {
        self.ssl_required = ssl_required;
    }

    pub(super) fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }
}
