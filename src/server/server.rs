use super::channel_message::ChannelMessage;
use super::consumer::Consumer;
use super::productor::Productor;
use crate::config::{Config, ServerConfig};
use log::{error, info};
use std::io::Result as IoResult;
use std::net::{AddrParseError, SocketAddr};
use thiserror::Error;
use tokio::io::{split, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::spawn;
use tokio::sync::mpsc::channel;
use uuid::Uuid;

const CHANNEL_LENGTH: usize = 1024;

#[derive(Debug, Error)]
pub enum Error {
    #[error("addr parse `{0}`")]
    AddrParse(#[from] AddrParseError),
}

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
        let (sender, receiver) = channel::<ChannelMessage>(CHANNEL_LENGTH);

        let mut consumer: Consumer = Consumer::new(self.config);
        consumer.set_recevier(receiver);

        // 生成链接的client_id
        let mut client_id: usize = 0;

        loop {
            select! {
                result = listener.accept() => {
                    match result {
                        Ok((socket, addr)) => {
                            let uuid: Uuid = Uuid::new_v4();
                            let (read_stream, write_stream): (ReadHalf<TcpStream>, WriteHalf<TcpStream>) = split(socket);

                            let product: Productor = Productor::new(read_stream, sender.clone(), uuid, self.add, addr);

                            if let Err(e) = consumer.add(uuid, write_stream, self.add, addr, client_id).await {
                                error!("{:?}", e);
                            };

                            // 客户端id + 1
                            client_id += 1;

                            spawn(async move {
                                product.run().await;
                            });
                        },
                        Err(e) => {
                            error!("{:?}", e);
                        }
                    }
                }
                _ = consumer.run() => {

                }
            }
        }
    }
}
