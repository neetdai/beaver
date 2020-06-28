use super::decode::{Decode, Error, Message};
use super::encode::{Info, Msg, Ping, Pong, ResponseOk};
use super::read_stream::ReadStream;
use super::sub_list::SubList;
use super::write_stream::WriteStream;
use crate::config::Config;
use crate::config::ServerConfig;
use crate::global_static::CONFIG;
use log::{debug, error};
use std::io::Result as IoResult;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::Mutex;

const BUFF_SIZE: usize = 512;

type WStream = Arc<Mutex<WriteStream>>;

#[derive(Debug)]
pub(super) struct Service {
    read_stream: ReadStream,
    write_stream: WStream,
    decode: Decode,
    config: &'static Config,
    client_id: usize,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    sub_list: Arc<Mutex<SubList<(WStream, String, Option<u32>)>>>,
}

impl Service {
    pub(super) fn new(
        read_stream: ReadHalf<TcpStream>,
        write_stream: WriteHalf<TcpStream>,
        client_id: usize,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        sub_list: Arc<Mutex<SubList<(WStream, String, Option<u32>)>>>,
    ) -> Self {
        let read_stream: ReadStream = ReadStream::new(read_stream);
        let write_stream: Arc<Mutex<WriteStream>> =
            Arc::new(Mutex::new(WriteStream::new(write_stream)));

        let decode: Decode = Decode::new(BUFF_SIZE);

        Self {
            read_stream,
            write_stream,
            decode,
            config: &CONFIG,
            client_id,
            local_addr,
            remote_addr,
            sub_list,
        }
    }

    pub(super) async fn run(mut self) {
        debug!(
            "remote addr {} ==========> local addr {}",
            self.remote_addr, self.local_addr
        );
        let mut buffer: [u8; BUFF_SIZE] = [0; BUFF_SIZE];

        {
            let mut stream = self.write_stream.lock().await;

            let server: &ServerConfig = self.config.get_server();
            let info: Info = Info::new()
                .set_server_id(server.get_server_id().clone())
                .set_server_name(server.get_server_name().clone())
                .set_version(server.get_version().clone())
                .set_host(server.get_ip().clone())
                .set_port(server.get_port())
                .set_auth_required(server.get_auth_required())
                .set_ssl_required(server.get_ssl_required())
                .set_max_payload(server.get_max_payload())
                .set_proto(server.get_proto())
                .set_client_id(self.client_id)
                .set_client_ip(self.remote_addr.ip());

            match info.format() {
                Ok(result) => {
                    debug!("local addr {} send info", self.local_addr);

                    if let Err(e) = stream.write(result.as_bytes()).await {
                        error!("{:?}", e);
                        return;
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                    return;
                }
            }
        }

        'main: loop {
            let result = self.read_stream.read(&mut buffer).await;
            match result {
                Ok(size) => {
                    if size == 0 {
                        break 'main;
                    } else {
                        self.decode.set_buff(&buffer[..size]);

                        let start_decode = Instant::now();
                        'decode: loop {
                            match self.decode.decode() {
                                Ok(poll) => {
                                    debug!("decode {:?}", Instant::now().checked_duration_since(start_decode));
                                    match poll {
                                        Poll::Ready(message) => {
                                            match message {
                                                // 由于这里的message的参数都是借用的, 所以尽量在原地使用
                                                Message::Connect(conn_info) => {
                                                    debug!(
                                                        "remote addr {} send connect",
                                                        self.remote_addr
                                                    );

                                                    if let Some(ssl_require) = conn_info
                                                        .get("ssl_require")
                                                        .and_then(|v| v.as_bool())
                                                    {
                                                        self.set_ssl(ssl_require).await;
                                                    }

                                                    if let Some(verbose) = conn_info
                                                        .get("verbose")
                                                        .and_then(|v| v.as_bool())
                                                    {
                                                        self.set_verbose(verbose).await;
                                                    }

                                                    if let Err(e) = self.send_ok().await {
                                                        error!("{:?}", e);
                                                    }
                                                }
                                                Message::Sub(subject, group, sid) => {
                                                    debug!("remote addr {} send sub, subject {} sid {}", self.remote_addr, subject, sid);

                                                    let mut sub_list = self.sub_list.lock().await;
                                                    (*sub_list).subscribe(
                                                        subject.to_string(),
                                                        (
                                                            self.write_stream.clone(),
                                                            sid.to_string(),
                                                            None,
                                                        ),
                                                    );
                                                }
                                                Message::Pub(subject, reply_to, content) => {
                                                    debug!(
                                                        "remote addr {} pub subject {} content {}",
                                                        self.remote_addr, subject, content
                                                    );
                                                    {
                                                        let start = Instant::now();
                                                        
                                                        let mut sub_list =
                                                            self.sub_list.lock().await;
                                                        debug!("sub {:?}", Instant::now().checked_duration_since(start));

                                                        if let Some(list) = (*sub_list)
                                                            .get_subscribe_item(subject.to_string())
                                                        {
                                                            let mut remove_index: Vec<usize> =
                                                                Vec::new();
                                                            let msg = Msg::new(
                                                                subject, reply_to, content,
                                                            );

                                                            for (
                                                                index,
                                                                (write_stream, sid, max_message),
                                                            ) in list.iter_mut().enumerate()
                                                            {
                                                                let start_lock = Instant::now();
                                                                let mut stream =
                                                                    write_stream.lock().await;

                                                                debug!("lock {:?}", Instant::now().checked_duration_since(start_lock));

                                                                let start_write = Instant::now();
                                                                if let Err(e) = stream
                                                                    .write(
                                                                        &msg.format(sid.as_str()),
                                                                    )
                                                                    .await
                                                                {
                                                                    error!("{:?}", e);
                                                                }
                                                                debug!("write {:?}", Instant::now().checked_duration_since(start_write));

                                                                // 倒序标记删除的下标
                                                                if let Some(max) = max_message {
                                                                    if *max > 0 {
                                                                        *max -= 1;
                                                                    } else {
                                                                        remove_index
                                                                            .insert(0, index);
                                                                    }
                                                                }
                                                            }

                                                            let start_remove = Instant::now();
                                                            // 通过倒序删除数组相应位置
                                                            for index in remove_index {
                                                                list.remove(index);
                                                            }
                                                            debug!("remove {:?}", Instant::now().checked_duration_since(start_remove));
                                                        }
                                                    }
                                                    if let Err(e) = self.send_ok().await {
                                                        error!("{:?}", e);
                                                    }
                                                }
                                                Message::UnSub(sid, max_messages) => {
                                                    let mut sublist = self.sub_list.lock().await;
                                                    sublist.remove_subscription(|(_, ssid, _)| {
                                                        ssid.eq(&sid)
                                                    });
                                                }
                                                Message::Pong => {
                                                    let mut stream = self.write_stream.lock().await;
                                                    if let Err(e) = stream
                                                        .write(Ping::format().as_bytes())
                                                        .await
                                                    {
                                                        error!("{:?}", e);
                                                    }
                                                }
                                                Message::Ping => {
                                                    let mut stream = self.write_stream.lock().await;
                                                    if let Err(e) = stream
                                                        .write(Pong::format().as_bytes())
                                                        .await
                                                    {
                                                        error!("{:?}", e);
                                                    }
                                                }
                                            }
                                            self.decode.reset();
                                        }
                                        Poll::Pending => {
                                            break 'decode;
                                        }
                                    }
                                }
                                Err(e) => {}
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                    break 'main;
                }
            }
        }
    }

    async fn set_ssl(&mut self, ssl_required: bool) {
        self.read_stream.set_ssl(ssl_required);
        let mut stream = self.write_stream.lock().await;
        stream.set_ssl(ssl_required);
    }

    async fn set_verbose(&mut self, verbose: bool) {
        let mut stream = self.write_stream.lock().await;
        stream.set_verbose(verbose);
    }

    async fn send_ok(&mut self) -> IoResult<()> {
        let mut stream = self.write_stream.lock().await;
        stream.send_ok(ResponseOk::format().as_bytes()).await
    }
}
