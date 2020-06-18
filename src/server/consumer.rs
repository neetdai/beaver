use super::channel_message::ChannelMessage;
use super::decode::Message;
use super::encode;
use super::sub_list::SubList;
use super::write_stream::WriteStream;
use crate::config::Config;
use async_spmc::Receiver as SubListReceiver;
use futures::future::join_all;
use futures::future::poll_fn;
use log::{debug, error, info};
use serde_json::Error as SerdeJsonError;
use std::collections::HashMap;
use std::future::Future;
use std::io::Error as IoError;
use std::mem::replace;
use std::net::SocketAddr;
use std::ops::Drop;
use std::pin::Pin;
use std::task::Poll;
use thiserror::Error;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::select;
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
    recevier: Receiver<ChannelMessage>,
    config: &'a Config,
    sid_map: HashMap<String, (Uuid, SubListReceiver<String>, Option<u32>)>,
    sub_list: SubList<String>,
}

impl<'a> Consumer<'a> {
    pub(super) fn new(config: &'a Config, recevier: Receiver<ChannelMessage>) -> Self {
        Self {
            map: HashMap::new(),
            recevier,
            config,
            sid_map: HashMap::new(),
            sub_list: SubList::new(),
        }
    }

    pub(super) async fn add(
        &mut self,
        uuid: Uuid,
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

        self.map.insert(uuid, write_stream);
        Ok(())
    }

    pub(super) async fn run(&mut self) {
        loop {
            if let Some(channel_message) = self.recevier.recv().await {
                match channel_message {
                    ChannelMessage::Shutdown(uuid) => {
                        self.map.remove(&uuid);

                        self.sid_map.retain(|_, item| item.0 != uuid);

                        debug!("map.len {:?}, sid.len {:?}", self.map.len(), self.sid_map.len());
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

                            // 注册订阅指定的主题
                            let receiver: SubListReceiver<String> =
                                self.sub_list.subscribe(subject);
                            self.sid_map.insert(sid, (uuid, receiver, None));

                            if let Err(e) = self.send_ok(&uuid).await {
                                error!("{:?}", e);
                            }
                        }
                        Message::UnSub(sid, max_message) => {
                            if max_message.is_some() {
                                if let Some((_, _, max)) = self.sid_map.get_mut(&sid) {
                                    *max = max_message;
                                }
                            } else {
                                self.sid_map.remove(&sid);
                            }

                            if let Err(e) = self.send_ok(&uuid).await {
                                error!("{:?}", e);
                            }
                        }
                        Message::Pub(subject, reply_to, content) => {
                            self.sub_list.send(subject.clone(), content);

                            if let Err(e) = self
                                .send_all_message_to_receiver(subject.as_str(), &reply_to)
                                .await
                            {
                                error!("{:?}", e);
                            }

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

    // 遍历这个sid_map中所有的receiver, 挑选出可以返回的接收者
    // 这里可以优化, 由于123行中, sub_list.send() 是单生产者多消费者模型
    // 那么就有可能出现, 只有一个消费者接受到消息, 但是需要遍历整个 sid_map 的情况
    async fn send_all_message_to_receiver(
        &mut self,
        subject: &str,
        reply_to: &Option<String>,
    ) -> Result<(), IoError> {
        let mut list = Vec::new();
        for (sid, (uuid, ref recv, max)) in self.sid_map.iter_mut() {
            debug!("wait sid {:?}", &sid);
            let poll_result = poll_fn(|cx| {
                let mut fut = recv.recv_iter();
                let fut = unsafe { Pin::new_unchecked(&mut fut) };

                let result = fut.poll(cx);

                debug!("poll result {:?}", result.is_ready());

                match result {
                    Poll::Ready(try_iter) => Poll::Ready(Some(try_iter)),
                    Poll::Pending => Poll::Ready(None),
                }
            })
            .await;

            // let try_iter = poll_result;
            if let Some(try_iter) = poll_result {
                debug!("{:?}", &sid);
                list.push((sid, uuid, try_iter, max));
            }
        }

        println!("publish list len {:?}", list.len());
        for (sid, uuid, try_iter, max) in list {
            if let Some(stream) = self.map.get_mut(uuid) {
                for message in try_iter {
                    stream
                        .write(
                            encode::Msg::new(
                                subject,
                                sid.as_str(),
                                reply_to.as_ref().map(|reply| reply.as_str()),
                                message.as_str(),
                            )
                            .format()
                            .as_bytes(),
                        )
                        .await?;

                    if let Some(total) = max {
                        *total -= 1;

                        if *total == 0 {
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
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
