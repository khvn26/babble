use crate::transport::types::{Channel, ConnState, User};

#[derive(Clone, Debug)]
pub struct TextMessage {
    pub actor_id: Option<u32>,
    pub channel_id: Option<u32>,
    pub user_ids: Vec<u32>,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum TransportEvent {
    ConnectionState(ConnState),
    Channels(Vec<Channel>),
    Users(Vec<User>),
    Text(TextMessage),
    Error(String),
}
