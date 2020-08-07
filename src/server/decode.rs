use bytes::{Buf, BufMut, BytesMut};
use serde;
use serde_derive::Deserialize;
use serde_json::{self, Error as SerdeError};
use std::io::Error as IoError;
use std::mem::swap;
use std::str::from_utf8;
use std::str::FromStr;
use std::str::Utf8Error;
use std::task::Poll;
use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum Error {
    #[error("parse error")]
    Parse,

    #[error("serde json error `{0}`")]
    Serde(#[from] SerdeError),

    #[error("utf8 error `{0}`")]
    Utf8(#[from] Utf8Error),

    #[error("unknow protocol")]
    UnknownProtocol,
}

#[derive(Debug)]
enum State {
    ConnectPrepare,
    ConnectSpace,
    Conn,
    Start,
    SubSpace,
    SubPrepare,
    PubSpace,
    PubComplete,
    Ping,
    PingPrepare,
    Pong,
    PongPrepare,
    UnSu,
    UnSubPrepare,
}

// todo: 一个存储Connect信息的结构体
// 由于json结构是可以变化的, 每一次读取都有可能出现某些字段会有, 某些字段会没有

// #[derive(Debug, Deserialize, PartialEq, Default)]
// pub(super) struct Connect {
//     name: String,
//     verbose: bool,
//     ssl_required: bool,
//     auth_required: bool,
//     user: Option<String>,
//     pass: Option<String>,
//     lang: String,
//     version: String,
// }

#[derive(Debug, PartialEq)]
pub(super) enum Message<'a> {
    Connect(serde_json::Value),
    Sub(&'a str, Option<&'a str>, &'a str),
    Pub(&'a str, Option<&'a str>, &'a str),
    UnSub(&'a str, Option<u32>),
    Pong,
    Ping,
}

// 解析nats协议
// 参考文档: https://www.cnblogs.com/vimisky/p/4931015.html
// 这里我想做一个解析, 我想了有两种方法:
// 一种是无状态的解析:
// 优点在于不需要操作内存去记录当前的状态, 对频繁调用的性能提升会很好
// 缺点在于解析PUB这种不友好, 因为 PUB <subject> [reply-to] <#bytes>\r\n[payload]\r\n 首先需要找到第一个\r, 然后再继续判断下去
// 当字节流没有一次性送完两个\r的情况下, 要不停的重复判断第一个\r, 扫描做无用工
// 
// 一种是有状态的解析:
// 优点和缺点刚好和无状态相反
// 
// 那么我这里是选择了有状态的写法. 那么要性能比较高的话, 尽量少分配内存.
// 本着少分配内存的原则, 所以我干脆也不创建一个新的变量作为返回, 直接给出引用
//
// 目前解析PUB长度为10000的字节流性能为:
// time:   [14.538 us 14.591 us 14.652 us]                        
// change: [+1.3137% +1.9753% +2.6860%] (p = 0.00 < 0.05)
// Performance has regressed.
// 
// 但是短字节流的性能比不上原版的nats
#[derive(Debug)]
pub(super) struct Decode {
    state: State,
    buff: BytesMut,
    end: usize,
}

impl Decode {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            state: State::Start,
            buff: BytesMut::with_capacity(capacity),
            end: 0,
        }
    }

    // 其实这里是没有考虑到接收的字节流是否会大于usize的大小, 
    // 或者说没有考虑到整个服务是否能撑住
    pub(super) fn set_buff(&mut self, buff: &[u8]) {
        self.buff.extend_from_slice(buff);
    }

    // 在解析的过程中,
    // 有可能会出现windows客户端发\r\n的换行符,
    // 而不是 \n
    pub(super) fn decode(&mut self) -> Result<Poll<Message>, Error> {
        loop {
            if self.buff.has_remaining() {
                if let Some(position) = self.buff[self.end..].iter().position(|item| *item == b'\n') {
                    self.end += position;
                    match self.state {
                        State::Start => {
                            if position >= 4 {
                                match &self.buff[..4] {
                                    b"CONN" => self.state = State::Conn,
                                    b"SUB " => self.state = State::SubSpace,
                                    b"PUB " => self.state = State::PubSpace,
                                    b"PING" => self.state = State::Ping,
                                    b"PONG" => self.state = State::Pong,
                                    b"UNSU" => self.state = State::UnSu,
                                    _ => return Err(Error::Parse),
                                }
                            } else {
                                return Ok(Poll::Pending);
                            }
                        }
                        State::Conn => {
                            if self.buff.len() < 8 {
                                self.end = self.buff.len();
                                return Ok(Poll::Pending);
                            }
                            if self.end >= 8 && &self.buff[4..8] == b"ECT " {
                                self.state = State::ConnectSpace;
                            } else {
                                return Err(Error::Parse);
                            }
                        }
                        State::ConnectSpace => {
                            if self.buff[self.end - 1] == b'\r' {
                                return self.connect_message_complete(8, self.end - 1);
                            } else {
                                return self.connect_message_complete(8, self.end);
                            }
                        }
                        State::SubSpace => {
                            if self.buff[self.end - 1] == b'\r' {
                                return self.sub_message_complete(4, self.end - 1);
                            } else {
                                return self.sub_message_complete(4, self.end);
                            }
                        }
                        State::Ping => return self.ping_message_complete(),
                        State::Pong => return self.pong_message_complete(),
                        State::PubSpace => {
                            if let Some(position) = self
                                .buff[(self.end + 1) ..]
                                .iter()
                                .position(|item| *item == b'\n')
                            {
                                self.end += position + 1;
                                return self.pub_complete(4, self.end);
                            } else {
                                return Ok(Poll::Pending);
                            }
                        }
                        State::UnSu => {
                            if self.buff.len() < 6 {
                                self.end = self.buff.len();
                                return Ok(Poll::Pending);
                            }
                            if self.end >= 6 && &self.buff[4..6] == b"B " {
                                self.state = State::UnSubPrepare;
                            } else {
                                return Err(Error::Parse);
                            }
                        }
                        State::UnSubPrepare => {
                            if self.buff[self.end - 1] == b'\r' {
                                return self.unsub_complete(6, self.end - 1);
                            } else {
                                return self.unsub_complete(6, self.end);
                            }
                        }
                        _ => return Err(Error::Parse),
                    }
                } else {
                    return Ok(Poll::Pending);
                }
            } else {
                return Ok(Poll::Pending);
            }
        }
    }

    fn sub_message(&self, start: usize, end: usize) -> Result<Message, Error> {
        let sub: Vec<&str> = {
            from_utf8(&self.buff[start..end])?
                .split_whitespace()
                .collect()
        };
        match sub[..] {
            [subject, sid] => Ok(Message::Sub(subject, None, sid)),
            [subject, group, sid] => Ok(Message::Sub(subject, Some(group), sid)),
            _ => Err(Error::Parse),
        }
    }

    fn connect_message(&self, start: usize, end: usize) -> Result<Message, Error> {
        from_utf8(&self.buff[start..end])
            .map_err(Error::Utf8)
            .and_then(|result| Ok(Message::Connect(serde_json::from_str(result)?)))
    }

    fn pong_message(&self) -> Message {
        Message::Pong
    }

    fn ping_message(&self) -> Message {
        Message::Ping
    }

    // 这里不能在decode的范围里面使用, 因为其中需要切割buff, 但是decode是返回引用buff的值, 所以应该是编译不过的.
    // 需要使用完decode返回值才调用reset
    pub(super) fn reset(&mut self) {
        self.state = State::Start;
        self.buff.split_to(self.end + 1);
        self.end = 0;
    }

    fn sub_message_complete(&mut self, start: usize, end: usize) -> Result<Poll<Message>, Error> {
        // 处理订阅
        self.sub_message(start, end).map(Poll::Ready)
    }

    fn connect_message_complete(
        &mut self,
        start: usize,
        end: usize,
    ) -> Result<Poll<Message>, Error> {
        self.connect_message(start, end).map(Poll::Ready)
    }

    fn pong_message_complete(&mut self) -> Result<Poll<Message>, Error> {
        Ok(Poll::Ready(self.pong_message()))
    }

    fn ping_message_complete(&mut self) -> Result<Poll<Message>, Error> {
        Ok(Poll::Ready(self.ping_message()))
    }

    fn pub_message(&mut self, start: usize, end: usize) -> Result<Poll<Message>, Error> {
        let params: Vec<&str> = {
            from_utf8(&self.buff[start..end])
                .map_err(Error::Utf8)?
                .trim()
                .split("\r\n")
                .take(2)
                .collect()
        };

        match params[..] {
            [info, content] => {
                let pub_result: Vec<&str> = info.split_whitespace().take(3).collect();
                match pub_result[..] {
                    [subject, content_length] => {
                        let content_length: usize =
                            usize::from_str(content_length).map_err(|_| Error::Parse)?;

                        if content_length == content.len() {
                            Ok(Poll::Ready(Message::Pub(subject, None, content)))
                        } else {
                            Err(Error::Parse)
                        }
                    }
                    [subject, replay, content_length] => {
                        let content_length: usize =
                            usize::from_str(content_length).map_err(|_| Error::Parse)?;

                        if content_length == content.len() {
                            Ok(Poll::Ready(Message::Pub(subject, Some(replay), content)))
                        } else {
                            Err(Error::Parse)
                        }
                    }
                    _ => Err(Error::Parse),
                }
            }
            _ => Err(Error::Parse),
        }
    }

    fn pub_complete(&mut self, start: usize, end: usize) -> Result<Poll<Message>, Error> {
        self.pub_message(start, end)
    }

    fn unsub_message(&mut self, start: usize, end: usize) -> Result<Poll<Message>, Error> {
        from_utf8(&self.buff[start..end])
            .map_err(Error::Utf8)
            .and_then(|unsub_message| {
                let result: Vec<&str> = unsub_message.trim().split_whitespace().take(2).collect();

                match result[..] {
                    [sid] => Ok(Poll::Ready(Message::UnSub(sid, None))),
                    [sid, max_msgs_message] => {
                        if let Ok(max_message) = u32::from_str(max_msgs_message) {
                            Ok(Poll::Ready(Message::UnSub(sid, Some(max_message))))
                        } else {
                            Err(Error::Parse)
                        }
                    }
                    _ => Err(Error::Parse),
                }
            })
    }

    fn unsub_complete(&mut self, start: usize, end: usize) -> Result<Poll<Message>, Error> {
        self.unsub_message(start, end)
    }
}

#[test]
fn decode_pong() {
    let mut decode = Decode::new(512);

    // windows的客户端
    decode.set_buff(b"PONG\r\n");

    let result = decode.decode();
    if let Ok(Poll::Ready(message)) = result {
        assert_eq!(message, Message::Pong);
    } else {
        panic!("message parse error");
    }

    decode.reset();

    // linux的客户端
    decode.set_buff(b"PONG\n");

    let result = decode.decode();
    if let Ok(Poll::Ready(message)) = result {
        assert_eq!(message, Message::Pong);
    } else {
        panic!("message parse error");
    }

    decode.reset();

    // 区分大小写
    decode.set_buff(b"pong\r\n");
    assert_eq!(decode.decode().is_err(), true);
}

#[test]
fn decode_ping_linux() {
    let mut decode = Decode::new(512);

    // linux的客户端
    decode.set_buff(b"PING\n");
    let result = decode.decode();
    if let Ok(Poll::Ready(message)) = result {
        assert_eq!(message, Message::Ping);
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
fn decode_ping_windows() {
    let mut decode = Decode::new(512);

    // windows的客户端
    decode.set_buff(b"PING\r\n");
    let result = decode.decode();
    if let Ok(Poll::Ready(message)) = result {
        assert_eq!(message, Message::Ping);
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
fn decode_connect_windows() {
    let mut decode = Decode::new(512);
    decode.set_buff(b"CONNECT {\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}\r\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Connect(message))) = result {
        // assert_eq!(message, Connect {
        //     name: "#rustlang".to_string(),
        //     verbose: true,
        //     ssl_required: false,
        //     auth_required: false,
        //     user: None,
        //     pass: None,
        //     lang: String::new(),
        //     version: String::new(),
        // });
        assert_eq!(
            message,
            serde_json::from_str::<serde_json::Value>(
                "{\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}"
            )
            .unwrap()
        );
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
fn decode_connect_linux() {
    let mut decode = Decode::new(512);
    decode.set_buff(b"CONNECT {\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Connect(message))) = result {
        // assert_eq!(message, Connect {
        //     name: "#rustlang".to_string(),
        //     verbose: true,
        //     ssl_required: false,
        //     auth_required: false,
        //     user: None,
        //     pass: None,
        //     lang: String::new(),
        //     version: String::new(),
        // });
        assert_eq!(
            message,
            serde_json::from_str::<serde_json::Value>(
                "{\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}"
            )
            .unwrap()
        );
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
fn decode_connect_chunks() {
    let mut decode = Decode::new(512);

    // 允许分开接收数据
    decode.set_buff(b"CONNE");
    let result = decode.decode();

    assert!(result.unwrap().is_pending());
    decode.set_buff(b"CT {\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}\r\n");
    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Connect(message))) = result {
        // assert_eq!(message, Connect {
        //     name: "#rustlang".to_string(),
        //     verbose: true,
        //     ssl_required: false,
        //     auth_required: false,
        //     user: None,
        //     pass: None,
        //     lang: String::new(),
        //     version: String::new(),
        // });
        assert_eq!(
            message,
            serde_json::from_str::<serde_json::Value>(
                "{\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}"
            )
            .unwrap()
        );
    } else {
        panic!("message parse error");
    }

    decode.reset();

    // 分开接收数据也是要合乎解析
    decode.set_buff(b"CONN");
    assert!(decode.decode().unwrap().is_pending());
    decode.set_buff(b"ect asdfasd\r\n");
    assert!(decode.decode().is_err());

    decode.reset();
}

#[test]
#[should_panic]
fn decode_connect_error() {
    let mut decode = Decode::new(512);
    decode.set_buff(b"connect {\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}\r\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Connect(message))) = result {
        // assert_eq!(message, Connect {
        //     name: "#rustlang".to_string(),
        //     verbose: true,
        //     ssl_required: false,
        //     auth_required: false,
        //     user: None,
        //     pass: None,
        //     lang: String::new(),
        //     version: String::new(),
        // });
        assert_eq!(
            message,
            serde_json::from_str::<serde_json::Value>(
                "{\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}"
            )
            .unwrap()
        );
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
fn decode_sub() {
    let mut decode = Decode::new(512);

    // 忽略订阅队列
    decode.set_buff(b"SUB asdfasd sdfaf\r\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Sub(subject, group, sid))) = result {
        assert_eq!(subject, "asdfasd");
        assert!(group.is_none());
        assert_eq!(sid, "sdfaf");
    } else {
        panic!("message parse error");
    }

    decode.reset();

    // 有订阅队列
    decode.set_buff(b"SUB asdfasd sdfds sdfaf\r\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Sub(subject, group, sid))) = result {
        assert_eq!(subject, "asdfasd");
        assert_eq!(group, Some("sdfds"));
        assert_eq!(sid, "sdfaf");
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
fn decode_sub_linux() {
    let mut decode = Decode::new(512);

    // 忽略订阅队列
    decode.set_buff(b"SUB asdfasd sdfaf\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Sub(subject, group, sid))) = result {
        assert_eq!(subject, "asdfasd");
        assert!(group.is_none());
        assert_eq!(sid, "sdfaf");
    } else {
        panic!("message parse error");
    }

    decode.reset();

    // 有订阅队列
    decode.set_buff(b"SUB asdfasd sdfds sdfaf\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Sub(subject, group, sid))) = result {
        assert_eq!(subject, "asdfasd");
        assert_eq!(group, Some("sdfds"));
        assert_eq!(sid, "sdfaf");
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
#[should_panic]
fn decode_sub_protocol_error() {
    let mut decode = Decode::new(512);
    // 协议不对
    decode.set_buff(b"sub asdfasd sdfds sdfaf\n");
    let result = decode.decode();
    result.unwrap();

    decode.reset();
}

#[test]
#[should_panic]
fn decode_sub_error() {
    let mut decode = Decode::new(512);
    // 格式不对
    decode.set_buff(b"SUB asdfasd asdfasdf sdfds sdfaf\n");
    let result = decode.decode();
    result.unwrap();

    decode.reset();
}

#[test]
fn decode_pub_message() {
    let mut decode = Decode::new(512);
    decode.set_buff(b"PUB FOO 11\r\nHello NATS!\r\n");
    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Pub(subject, reply, content))) = result {
        assert_eq!(subject, "FOO");
        assert_eq!(reply, None);
        assert_eq!(content, "Hello NATS!");
    } else {
        panic!("message parse error");
    }

    decode.reset();

    decode.set_buff(b"PUB FOO sdfsa 11\r\nHello World\r\nPUB F= 12\r\nHello World!\r\n");
    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Pub(subject, reply, content))) = result {
        assert_eq!(subject, "FOO");
        assert_eq!(reply, Some("sdfsa"));
        assert_eq!(content, "Hello World");
    } else {
        panic!("message parse error");
    }

    decode.reset();

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Pub(subject, reply, content))) = result {
        assert_eq!(subject, "F=");
        assert_eq!(reply, None);
        assert_eq!(content, "Hello World!");
    } else {
        panic!("message parse error");
    }

    decode.reset();

    decode.set_buff(b"PUB FOO");
    decode.set_buff(b" 11\r\nHello NATS!\r\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::Pub(subject, reply, content))) = result {
        assert_eq!(subject, "FOO");
        assert_eq!(reply, None);
        assert_eq!(content, "Hello NATS!");
    } else {
        panic!("message parse error");
    }

    decode.reset();
}

#[test]
fn decode_unsub_message() {
    let mut decode = Decode::new(512);

    decode.set_buff(b"UNSUB hello\r\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::UnSub(sid, max_message))) = result {
        assert_eq!(sid, "hello");
        assert_eq!(max_message, None);
    } else {
        panic!("message parse error");
    }

    decode.reset();

    decode.set_buff(b"UNSUB hello 5\r\n");

    let result = decode.decode();

    if let Ok(Poll::Ready(Message::UnSub(sid, max_message))) = result {
        assert_eq!(sid, "hello");
        assert_eq!(max_message, Some(5));
    } else {
        panic!("message parse error");
    }

    decode.reset();
}
