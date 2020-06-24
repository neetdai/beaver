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
    Sub,
    Su,
    S,
    Pub,
    Pu,
    P,
    Pin,
    Pi,
    Pon,
    Po,
    UnSu,
    UnS,
    Un,
    U,
    ConnectPrepare,
    ConnectSpace,
    Connec,
    Conne,
    Conn,
    Con,
    Co,
    C,

    Start,
    SubSpace,
    SubPrepare,
    PubSpace,
    PubComplete,
    Ping,
    PingPrepare,
    Pong,
    PongPrepare,
    UnSub,
    UnSubPrepare,
    Info,
    Connect,
    Msg,
    Ok,
    Err,
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

#[derive(Debug)]
pub(super) struct Decode {
    state: State,
    buff: BytesMut,
    params: Vec<u8>,
}

impl Decode {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            state: State::Start,
            buff: BytesMut::with_capacity(capacity),
            params: Vec::new(),
        }
    }

    pub(super) fn set_buff(&mut self, buff: &[u8]) {
        self.buff.put(buff);
    }

    // 在解析的过程中,
    // 有可能会出现windows客户端发\r\n的换行符,
    // 而不是 \n
    pub(super) fn decode(&mut self) -> Result<Poll<Message>, Error> {
        loop {
            if self.buff.has_remaining() {
                let item = self.buff.get_u8();
                match self.state {
                    State::Start => match item {
                        b'C' => self.state = State::C,
                        b'P' => self.state = State::P,
                        b'S' => self.state = State::S,
                        b'U' => self.state = State::U,
                        _ => {
                            // self.reset();
                            return Err(Error::Parse);
                        }
                    },
                    State::C if item == b'O' => self.state = State::Co,
                    State::Co if item == b'N' => self.state = State::Con,
                    State::Con if item == b'N' => self.state = State::Conn,
                    State::Conn if item == b'E' => self.state = State::Conne,
                    State::Conne if item == b'C' => self.state = State::Connec,
                    State::Connec if item == b'T' => self.state = State::Connect,
                    State::Connect if item == b' ' || item == b'\t' => {
                        self.state = State::ConnectSpace;
                    }
                    State::ConnectSpace => match item {
                        b'\r' => {
                            self.state = State::ConnectPrepare;
                        }
                        b'\n' => {
                            return self.connect_message_complete();
                        }
                        _ => {
                            self.params.push(item);
                        }
                    },
                    State::ConnectPrepare => match item {
                        b'\n' => {
                            return self.connect_message_complete();
                        }
                        _ => {
                            self.params.push(b'\r');
                            self.params.push(item);
                            self.state = State::ConnectSpace;
                        }
                    },
                    State::P => match item {
                        b'I' => self.state = State::Pi,
                        b'O' => self.state = State::Po,
                        b'U' => self.state = State::Pu,
                        _ => {
                            return Err(Error::Parse);
                        }
                    },
                    State::Pi if item == b'N' => self.state = State::Pin,
                    State::Pin if item == b'G' => self.state = State::Ping,
                    State::Ping => match item {
                        b'\r' => self.state = State::PingPrepare,
                        b'\n' => {
                            return self.ping_message_complete();
                        }
                        _ => return Err(Error::UnknownProtocol),
                    },
                    State::PingPrepare => {
                        if item == b'\n' {
                            return self.ping_message_complete();
                        } else {
                            return Err(Error::UnknownProtocol);
                        }
                    }
                    State::Po if item == b'N' => self.state = State::Pon,
                    State::Pon if item == b'G' => self.state = State::Pong,
                    State::Pong => match item {
                        b'\n' => {
                            return self.pong_message_complete();
                        }
                        b'\r' => {
                            self.state = State::PongPrepare;
                        }
                        _ => {
                            return Err(Error::UnknownProtocol);
                        }
                    },
                    State::PongPrepare => {
                        if item == b'\n' {
                            return self.pong_message_complete();
                        } else {
                            return Err(Error::UnknownProtocol);
                        }
                    }
                    State::S if item == b'U' => self.state = State::Su,
                    State::Su if item == b'B' => self.state = State::Sub,
                    State::Sub if item == b' ' || item == b'\r' => self.state = State::SubPrepare,
                    State::SubSpace => match item {
                        b'\r' => {
                            self.state = State::SubPrepare;
                        }
                        b'\n' => {
                            return self.sub_message_complete();
                        }
                        _ => {
                            self.params.push(item);
                        }
                    },
                    State::SubPrepare => match item {
                        b'\n' => {
                            return self.sub_message_complete();
                        }
                        _ => {
                            self.params.push(b'\r');
                            self.params.push(item);
                            self.state = State::SubSpace;
                        }
                    },
                    State::Pu if item == b'B' => self.state = State::Pub,
                    State::Pub => match item {
                        b' ' => {
                            self.state = State::PubSpace;
                        }
                        b'\r' => {}
                        _ => {}
                    },
                    State::PubSpace => match item {
                        b'\n' => {
                            self.params.push(item);
                            self.state = State::PubComplete;
                        }
                        _ => {
                            self.params.push(item);
                        }
                    },
                    State::PubComplete => match item {
                        b'\n' => {
                            self.params.push(item);
                            return self.pub_complete();
                        }
                        _ => {
                            self.params.push(item);
                        }
                    },
                    State::U if item == b'N' => self.state = State::Un,
                    State::Un if item == b'S' => self.state = State::UnS,
                    State::UnS if item == b'U' => self.state = State::UnSu,
                    State::UnSu if item == b'B' => self.state = State::UnSub,
                    State::UnSub if item == b' ' || item == b'\t' => {
                        self.state = State::UnSubPrepare
                    }
                    State::UnSubPrepare => match item {
                        b'\n' => {
                            self.params.push(item);
                            return self.unsub_complete();
                        }
                        _ => {
                            self.params.push(item);
                        }
                    },
                    _ => {
                        // self.reset();
                        return Err(Error::Parse);
                    }
                }
            } else {
                return Ok(Poll::Pending);
            }
        }
    }

    fn sub_message(&self) -> Result<Message, Error> {
        let sub: Vec<&str> = { from_utf8(&self.params)?.split_whitespace().collect() };
        match sub[..] {
            [subject, sid] => Ok(Message::Sub(subject, None, sid)),
            [subject, group, sid] => Ok(Message::Sub(subject, Some(group), sid)),
            _ => Err(Error::Parse),
        }
    }

    fn connect_message(&self) -> Result<Message, Error> {
        from_utf8(&self.params)
            .map_err(Error::Utf8)
            .and_then(|result| Ok(Message::Connect(serde_json::from_str(result)?)))
    }

    fn pong_message(&self) -> Message {
        Message::Pong
    }

    fn ping_message(&self) -> Message {
        Message::Ping
    }

    pub(super) fn reset(&mut self) {
        self.state = State::Start;
        self.params.clear();
    }

    fn sub_message_complete(&mut self) -> Result<Poll<Message>, Error> {
        // 处理订阅
        let result = self.sub_message();
        // self.reset();
        result.map(Poll::Ready)
    }

    fn connect_message_complete(&mut self) -> Result<Poll<Message>, Error> {
        let result = self.connect_message();
        // self.reset();
        result.map(Poll::Ready)
    }

    fn pong_message_complete(&mut self) -> Result<Poll<Message>, Error> {
        let result = self.pong_message();
        // self.reset();
        Ok(Poll::Ready(result))
    }

    fn ping_message_complete(&mut self) -> Result<Poll<Message>, Error> {
        let result = self.ping_message();
        // self.reset();
        Ok(Poll::Ready(result))
    }

    fn pub_message(&mut self) -> Result<Poll<Message>, Error> {
        let params: Vec<&str> = {
            from_utf8(&self.params)
                .map_err(Error::Utf8)?
                .trim()
                .split("\r\n")
                .collect()
        };

        match params[..] {
            [info, content] => {
                let pub_result: Vec<&str> = info.split_whitespace().collect();
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

    fn pub_complete(&mut self) -> Result<Poll<Message>, Error> {
        let result = self.pub_message();
        // self.reset();
        result
    }

    fn unsub_message(&mut self) -> Result<Poll<Message>, Error> {
        from_utf8(&self.params)
            .map_err(Error::Utf8)
            .and_then(|unsub_message| {
                let result: Vec<&str> = unsub_message.trim().split_whitespace().collect();

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

    fn unsub_complete(&mut self) -> Result<Poll<Message>, Error> {
        let result = self.unsub_message();
        // self.reset();
        result
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
    decode.set_buff(b"CONN");
    let result = decode.decode();

    assert!(result.unwrap().is_pending());
    decode.set_buff(b"ECT {\"name\":\"#rustlang\",\"pedantic\":false,\"verbose\":true}\r\n");
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
