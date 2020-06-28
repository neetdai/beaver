use super::service::Service;
use super::sub_list::SubList;
use crate::config::{Config, ServerConfig};
use crate::global_static::CONFIG;
use log::{error, info};
use std::io::Result as IoResult;
use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{split, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum Error {
    #[error("addr parse `{0}`")]
    AddrParse(#[from] AddrParseError),
}

pub struct Server {
    add: SocketAddr,
}

impl Server {
    pub fn new() -> Result<Server, Error> {
        let server_config: &ServerConfig = CONFIG.get_server();
        let addr: SocketAddr =
            format!("{}:{}", server_config.get_ip(), server_config.get_port()).parse()?;

        Ok(Self {
            add: addr,
        })
    }

    pub async fn run(self) -> IoResult<()> {
        let mut listener = TcpListener::bind(self.add).await?;
        let mut client_id: usize = 0;
        let sub_list = Arc::new(Mutex::new(SubList::new()));

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let (read_stream, write_stream): (ReadHalf<TcpStream>, WriteHalf<TcpStream>) =
                        split(socket);

                    let service: Service = Service::new(
                        read_stream,
                        write_stream,
                        client_id,
                        self.add,
                        addr,
                        sub_list.clone(),
                    );
                    client_id += 1;

                    spawn(service.run());
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }
    }
}
