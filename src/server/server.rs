use super::service::Service;
use crate::config::{Config, ServerConfig};
use log::{error, info};
use std::io::Result as IoResult;
use std::net::{AddrParseError, SocketAddr};
use thiserror::Error;
use tokio::io::{split, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::spawn;
use super::sub_list::SubList;
use tokio::sync::Mutex;
use std::sync::Arc;

const CHANNEL_LENGTH: usize = 1024;

#[derive(Debug, Error)]
pub enum Error {
    #[error("addr parse `{0}`")]
    AddrParse(#[from] AddrParseError),
}

type SubListShare = Arc<Mutex<SubList<(String, Option<String>, String)>>>;

pub struct Server<'a> {
    config: &'a Config,
    add: SocketAddr,
}

impl<'a> Server<'a> {
    pub fn new(config: &'a Config) -> Result<Server, Error> {
        let server_config: &ServerConfig = config.get_server();
        let addr: SocketAddr =
            format!("{}:{}", server_config.get_ip(), server_config.get_port()).parse()?;

        Ok(Self { config, add: addr })
    }

    pub async fn run(mut self) -> IoResult<()> {
        let mut listener = TcpListener::bind(self.add).await?;
        let mut client_id: usize = 0;
        let sub_list: SubListShare = Arc::new(
            Mutex::new(SubList::new())  
        );

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let (read_stream, write_stream): (ReadHalf<TcpStream>, WriteHalf<TcpStream>) =
                        split(socket);

                    let service: Service =
                        Service::new(read_stream, write_stream, client_id, self.add, addr, sub_list.clone());
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
