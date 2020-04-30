use super::channel_message::ChannelMessage;
use super::decode::{Decode, Message};
use log::{debug, info};
use std::net::Shutdown;
use std::net::SocketAddr;
use std::task::Poll;
use tokio::io::AsyncReadExt;
use tokio::io::ReadHalf;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::task::yield_now;
use uuid::Uuid;

#[derive(Debug)]
pub(super) struct ReadStream {
    decode: Decode,
    sender: Sender<ChannelMessage>,
    uuid: Uuid,
    stream: ReadHalf<TcpStream>,

    // 本地ip和端口
    local_addr: SocketAddr,

    // 客户端ip和端口
    remote_addr: SocketAddr,

    ssl_required: bool,
    auth_required: bool,
}

impl ReadStream {
    pub(super) fn new(
        stream: ReadHalf<TcpStream>,
        sender: Sender<ChannelMessage>,
        uuid: Uuid,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Self {
        Self {
            decode: Decode::new(512),
            sender,
            uuid,
            stream,
            ssl_required: false,
            auth_required: false,
            local_addr,
            remote_addr,
        }
    }

    pub(super) async fn run(mut self) {
        let mut buf: [u8; 512] = [0; 512];
        loop {
            match self.stream.read(&mut buf).await {
                // tcp好像是规定如果读取的数据长度为0时, 代表没有数据输入
                // 关闭tcp的读取之后要通知comsumer的tcp关闭写入
                Ok(size) if size == 0 => {
                    if let Err(e) = self.sender.try_send(ChannelMessage::Shutdown(self.uuid)) {
                        info!("read stream can'\t shutdown becase {}", e);
                    }
                    break;
                }

                Ok(size) => {
                    self.decode.set_buff(&buf[0..size]);
                    match self.decode.decode() {
                        Ok(poll) => {
                            match poll {
                                Poll::Ready(message) => {
                                    debug!(
                                        "{} ----------------> {} send message :{:?}",
                                        self.remote_addr, self.local_addr, message
                                    );
                                    // if let Err(e) = self
                                    //     .sender
                                    //     .send(ChannelMessage::Message(self.uuid, message))
                                    // {
                                    //     info!("{:?}", e);
                                    // }
                                    match message {
                                        Message::Connect(ref connect_info) => {
                                            self.ssl_required = connect_info
                                                .get("ssl_required")
                                                .and_then(|value| value.as_bool())
                                                .map_or(false, |value| value);

                                            self.auth_required = connect_info
                                                .get("auth_required")
                                                .and_then(|value| value.as_bool())
                                                .map_or(false, |value| value);

                                            if let Err(e) = self.sender.try_send(
                                                ChannelMessage::Message(self.uuid, message),
                                            ) {
                                                info!("{:?}", e);
                                            } else {
                                                debug!("send success");
                                            }
                                        }
                                        _ => {
                                            if let Err(e) = self.sender.try_send(
                                                ChannelMessage::Message(self.uuid, message),
                                            ) {
                                                info!("{:?}", e);
                                            } else {
                                                debug!("send success");
                                            }
                                        }
                                    }
                                }
                                Poll::Pending => {
                                    yield_now().await;
                                }
                            }
                        }
                        Err(e) => {
                            info!("{}", e);
                        }
                    }
                }
                Err(e) => {
                    info!("{}", e);
                }
            }
        }
    }
}
