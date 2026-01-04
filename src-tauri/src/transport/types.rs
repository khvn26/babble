#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub channel_id: u32,
    pub muted: bool,
    pub deafened: bool,
    pub talking: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}
