use super::channel_message::ChannelMessage;
use super::decode::Message;
use super::encode;
use super::sub_list::SubList;
use super::sub_struct::{MessageEvent, SubStruct};
use super::write_stream::WriteStream;
use crate::config::Config;
use async_spmc::Receiver as SubListReceiver;
use log::{debug, error, info};
use serde_json::Error as SerdeJsonError;
use std::collections::{BTreeMap, HashMap};
use std::io::Error as IoError;
use std::mem::replace;
use std::net::SocketAddr;
use std::ops::Drop;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub(super) enum ConsumerError {
    #[error("io error `{0}`")]
    Io(#[from] IoError),

    #[error("serde json `{0}`")]
    SerdeJson(#[from] SerdeJsonError),
}

#[derive(Debug)]
pub(super) struct Consumer<'a> {
    map: BTreeMap<u64, Arc<Mutex<WriteStream>>>,
    recevier: UnboundedReceiver<ChannelMessage>,
    config: &'a Config,
    sid_map: HashMap<String, UnboundedSender<MessageEvent>>,
    sub_list: SubList<(String, Option<String>, String)>,
}

impl<'a> Consumer<'a> {
    pub(super) fn new(config: &'a Config, recevier: UnboundedReceiver<ChannelMessage>) -> Self {
        Self {
            map: BTreeMap::new(),
            recevier,
            config,
            sid_map: HashMap::new(),
            sub_list: SubList::new(),
        }
    }

    pub(super) async fn add(
        &mut self,
        // uuid: Uuid,
        uuid: u64,
        stream: WriteHalf<TcpStream>,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        client_id: usize,
    ) -> Result<(), ConsumerError> {
        let server_config = self.config.get_server();

        let info = encode::Info::new()
            .set_host(server_config.get_ip().clone())
            .set_port(server_config.get_port())
            .set_server_id(server_config.get_server_id().clone())
            .set_server_name(server_config.get_server_name().clone())
            .set_version(server_config.get_version().clone())
            .set_auth_required(server_config.get_auth_required())
            .set_ssl_required(server_config.get_ssl_required())
            .set_max_payload(server_config.get_max_payload())
            .set_proto(server_config.get_proto())
            .set_client_id(client_id)
            .set_client_ip(remote_addr.ip());

        let mut write_stream: WriteStream = WriteStream::new(stream, local_addr, remote_addr);

        write_stream.write(info.format()?.as_bytes()).await?;

        self.map.insert(uuid, Arc::new(Mutex::new(write_stream)));
        Ok(())
    }

    pub(super) async fn run(&mut self) {
        loop {
            if let Some(channel_message) = self.recevier.recv().await {
                match channel_message {
                    ChannelMessage::Shutdown(uuid) => {
                        self.map.remove(&uuid);

                        info!(
                            "map.len {:?}, sid.len {:?} sublist total {:?}",
                            self.map.len(),
                            self.sid_map.len(),
                            self.sub_list.total()
                        );
                    }
                    ChannelMessage::Message(uuid, message) => match message {
                        Message::Connect(connect_info) => {
                            debug!("{:?}", connect_info);

                            if let Some(ssl_require) =
                                connect_info.get("ssl_require").and_then(|v| v.as_bool())
                            {
                                self.set_ssl_required(&uuid, ssl_require).await;
                            }

                            if let Some(verbose) =
                                connect_info.get("verbose").and_then(|v| v.as_bool())
                            {
                                debug!("{:?}", verbose);
                                self.set_verbose(&uuid, verbose).await;
                            }

                            if let Err(e) = self.send_ok(&uuid).await {
                                error!("{:?}", e);
                            }
                        }
                        Message::Sub(subject, group, sid) => {
                            debug!("sid {:?}", sid);

                            // 注册订阅指定的主题
                            let sub_receiver: SubListReceiver<(String, Option<String>, String)> =
                                self.sub_list.subscribe(subject);

                            let (sender, receiver): (
                                UnboundedSender<MessageEvent>,
                                UnboundedReceiver<MessageEvent>,
                            ) = unbounded_channel();

                            if let Some(stream) = self.map.get(&uuid) {
                                let sub_struct: SubStruct = SubStruct::new(
                                    stream.clone(),
                                    sub_receiver,
                                    receiver,
                                    sid.clone(),
                                );

                                self.sid_map.insert(sid, sender);

                                spawn(sub_struct.run());
                            }
                            if let Err(e) = self.send_ok(&uuid).await {
                                error!("{:?}", e);
                            }
                        }
                        Message::UnSub(sid, max_message) => {
                            if let Some(send) = self.sid_map.get_mut(&sid) {
                                if let Err(e) = send.send(MessageEvent::UnSub(max_message)) {
                                    error!("{:?}", e);
                                }
                            }

                            self.sid_map.remove(&sid);

                            if let Err(e) = self.send_ok(&uuid).await {
                                error!("{:?}", e);
                            }
                        }
                        Message::Pub(subject, reply_to, content) => {
                            self.sub_list
                                .send(subject.clone(), (content, reply_to, subject));

                            if let Err(e) = self.send_ok(&uuid).await {
                                error!("{:?}", e);
                            }
                        }
                        Message::Pong => {
                            if let Err(e) = self.send_ping(&uuid).await {
                                error!("{:?}", e);
                            }

                            // if let Err(e) = self.send_pong(&uuid).await {
                            //     error!("{:?}", e);
                            // }
                        }
                        Message::Ping => {
                            if let Err(e) = self.send_pong(&uuid).await {
                                error!("{:?}", e);
                            }

                            // if let Err(e) = self.send_ping(&uuid).await {
                            //     error!("{:?}", e);
                            // }
                        }
                    },
                }
            }
        }
    }

    async fn send_ping(&mut self, uuid: &u64) -> Result<(), IoError> {
        match self.map.get_mut(uuid) {
            Some(stream) => {
                (*stream.lock().await)
                    .write(encode::Ping::format().as_bytes())
                    .await?;
                Ok(())
            }
            None => {
                info!("stream uuid {} not found", uuid);
                Ok(())
            }
        }
    }

    async fn send_pong(&mut self, uuid: &u64) -> Result<(), IoError> {
        match self.map.get_mut(uuid) {
            Some(stream) => {
                (*stream.lock().await)
                    .write(encode::Pong::format().as_bytes())
                    .await?;
                Ok(())
            }
            None => {
                info!("stream uuid {} not found", uuid);
                Ok(())
            }
        }
    }

    async fn send_ok(&mut self, uuid: &u64) -> Result<(), IoError> {
        match self.map.get_mut(uuid) {
            Some(stream) => {
                (*stream.lock().await)
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

    async fn set_ssl_required(&mut self, uuid: &u64, ssl_require: bool) {
        if let Some(stream) = self.map.get_mut(uuid) {
            (*stream.lock().await).set_ssl(ssl_require);
        }
    }

    async fn set_verbose(&mut self, uuid: &u64, verbose: bool) {
        if let Some(stream) = self.map.get_mut(uuid) {
            (*stream.lock().await).set_verbose(verbose);
        }
    }
}

// impl<'a> Drop for Consumer<'a> {
//     fn drop(&mut self) {
//         let tmp = replace(&mut self.map, BTreeMap::new());

//         spawn(join_all(tmp.into_iter().map(
//             |(_, mut stream)| async move {
//                 match stream.shutdown().await {
//                     Ok(_) => info!("stream shutdown both"),
//                     Err(e) => error!("{:?}", e),
//                 };
//             },
//         )));
//     }
// }
