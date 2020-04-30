use super::channel_message::ChannelMessage;
use super::decode::Message;
use super::encode;
use super::write_stream::WriteStream;
use crate::config::Config;
use futures::future::join_all;
use log::{debug, error, info};
use serde_json::Error as SerdeJsonError;
use std::collections::HashMap;
use std::io::Error as IoError;
use std::mem::replace;
use std::net::SocketAddr;
use std::ops::Drop;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

#[derive(Debug, Error)]
pub(super) enum ConsumerError {
    #[error("io error `{0}`")]
    Io(#[from] IoError),

    #[error("serde json `{0}`")]
    SerdeJson(#[from] SerdeJsonError),
}

#[derive(Debug)]
pub(super) struct Consumer<'a> {
    map: HashMap<Uuid, WriteStream>,
    recevier: Option<Receiver<ChannelMessage>>,
    config: &'a Config,
}

impl<'a> Consumer<'a> {
    pub(super) fn new(config: &'a Config) -> Self {
        Self {
            map: HashMap::new(),
            recevier: None,
            config,
        }
    }

    pub(super) fn set_recevier(&mut self, recevier: Receiver<ChannelMessage>) {
        self.recevier = Some(recevier);
    }

    pub(super) async fn add(
        &mut self,
        uuid: Uuid,
        stream: WriteHalf<TcpStream>,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Result<(), ConsumerError> {
        let server_config = self.config.get_server();

        let info = encode::Info::new()
            .set_host(server_config.get_ip().clone())
            .set_port(server_config.get_port())
            .set_server_id(server_config.get_server_id().clone())
            .set_version(server_config.get_version().clone())
            .set_auth_required(server_config.get_auth_required())
            .set_ssl_required(server_config.get_ssl_required())
            .set_max_payload(server_config.get_max_payload());

        let mut write_stream: WriteStream = WriteStream::new(stream, local_addr, remote_addr);

        write_stream.write(info.format()?.as_bytes()).await?;

        self.map.insert(uuid, write_stream);
        Ok(())
    }

    pub(super) async fn run(&mut self) {
        loop {
            if let Some(recv) = &mut self.recevier {
                if let Some(channel_message) = recv.recv().await {
                    match channel_message {
                        ChannelMessage::Shutdown(uuid) => {
                            self.map.remove(&uuid);
                        }
                        ChannelMessage::Message(uuid, message) => match message {
                            Message::Connect(connect_info) => {
                                debug!("{:?}", connect_info);

                                if let Some(ssl_require) =
                                    connect_info.get("ssl_require").and_then(|v| v.as_bool())
                                {
                                    self.set_ssl_required(&uuid, ssl_require);
                                }

                                if let Some(verbose) =
                                    connect_info.get("verbose").and_then(|v| v.as_bool())
                                {
                                    debug!("{:?}", verbose);
                                    self.set_verbose(&uuid, verbose);
                                }

                                if let Err(e) = self.send_ok(&uuid).await {
                                    error!("{:?}", e);
                                }
                            }
                            Message::Sub(subject, group, sid) => {
                                debug!("sid {:?}", sid);
                            }
                            Message::Pub() => {}
                            Message::Pong => {
                                if let Err(e) = self.send_ping(&uuid).await {
                                    error!("{:?}", e);
                                }
                            }
                            Message::Ping => {
                                if let Err(e) = self.send_pong(&uuid).await {
                                    error!("{:?}", e);
                                }
                            }
                        },
                    }
                }
            }
        }
    }

    async fn send_ping(&mut self, uuid: &Uuid) -> Result<(), IoError> {
        match self.map.get_mut(uuid) {
            Some(stream) => {
                stream.write(encode::Ping::format().as_bytes()).await?;
                Ok(())
            }
            None => {
                info!("stream uuid {} not found", uuid);
                Ok(())
            }
        }
    }

    async fn send_pong(&mut self, uuid: &Uuid) -> Result<(), IoError> {
        match self.map.get_mut(uuid) {
            Some(stream) => {
                stream.write(encode::Pong::format().as_bytes()).await?;
                Ok(())
            }
            None => {
                info!("stream uuid {} not found", uuid);
                Ok(())
            }
        }
    }

    async fn send_ok(&mut self, uuid: &Uuid) -> Result<(), IoError> {
        match self.map.get_mut(uuid) {
            Some(stream) => {
                stream
                    .send_ok(encode::ResponseOk::format().as_bytes())
                    .await?;
                Ok(())
            }
            None => {
                info!("stream uuid {} not found", uuid);
                Ok(())
            }
        }
    }

    fn set_ssl_required(&mut self, uuid: &Uuid, ssl_require: bool) {
        if let Some(stream) = self.map.get_mut(uuid) {
            stream.set_ssl(ssl_require);
        }
    }

    fn set_verbose(&mut self, uuid: &Uuid, verbose: bool) {
        if let Some(stream) = self.map.get_mut(uuid) {
            stream.set_verbose(verbose);
        }
    }
}

impl<'a> Drop for Consumer<'a> {
    fn drop(&mut self) {
        let tmp = replace(&mut self.map, HashMap::new());

        spawn(join_all(tmp.into_iter().map(
            |(_, mut stream)| async move {
                match stream.shutdown().await {
                    Ok(_) => info!("stream shutdown both"),
                    Err(e) => error!("{:?}", e),
                };
            },
        )));
    }
}
