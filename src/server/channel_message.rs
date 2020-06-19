use super::decode::Message;
use uuid::Uuid;

#[derive(Debug)]
pub(super) enum ChannelMessage {
    Shutdown(u64),
    Message(u64, Message),
}
