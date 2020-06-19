use super::encode::Msg;
use super::write_stream::WriteStream;
use async_spmc::Receiver as SubReceiver;
use log::error;
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;

#[derive(Debug)]
pub(super) enum MessageEvent {
    UnSub(Option<u32>),
}

#[derive(Debug)]
pub(super) struct SubStruct {
    stream: Arc<Mutex<WriteStream>>,
    sub_receiver: SubReceiver<(String, Option<String>, String)>,
    receiver: UnboundedReceiver<MessageEvent>,
    sid: String,
    max_message: Option<u32>,
}

impl SubStruct {
    pub(super) fn new(
        stream: Arc<Mutex<WriteStream>>,
        sub_receiver: SubReceiver<(String, Option<String>, String)>,
        receiver: UnboundedReceiver<MessageEvent>,
        sid: String,
    ) -> Self {
        Self {
            stream,
            sub_receiver,
            receiver,
            sid,
            max_message: None,
        }
    }

    pub(super) async fn run(mut self) {
        'run: loop {
            select! {
                Some(event) =self.receiver.recv() => {
                    match event {
                        MessageEvent::UnSub(max_message) => {
                            if max_message.is_none() {
                                break 'run;
                            } else {
                                self.max_message = max_message;
                            }
                        }
                    }
                }
                try_iter = self.sub_receiver.recv_iter() => {
                    let mut stream = self.stream.lock().await;
                    for (content, reply_to, subject) in try_iter {

                        if let Err(e) = stream.write(
                            Msg::new(subject.as_str(), self.sid.as_str(), reply_to.as_deref(), content.as_str())
                                .format()
                                .as_bytes()
                        ).await {
                            error!("{:?}", e);
                            break 'run;
                        }

                        if let Some(count) = self.max_message.as_mut() {
                            *count += 1;

                            if *count == 0 {
                                break 'run;
                            }
                        }
                    }
                }
            }
        }
    }
}
