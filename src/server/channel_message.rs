use super::decode::Message;
use uuid::Uuid;

#[derive(Debug)]
pub(super) enum ChannelMessage {
    Shutdown(Uuid),
    Message(Uuid, Message),
}
