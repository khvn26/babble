use std::collections::HashMap;

use crate::transport::types::{Channel, User};

#[derive(Debug, Default)]
pub struct StateCache {
    channels: HashMap<u32, Channel>,
    users: HashMap<u32, User>,
}

#[derive(Debug)]
pub struct ChannelStateUpdate {
    pub id: u32,
    pub name: Option<String>,
    pub parent_id: Option<u32>,
}

#[derive(Debug)]
pub struct UserStateUpdate {
    pub id: u32,
    pub name: Option<String>,
    pub channel_id: Option<u32>,
    pub muted: Option<bool>,
    pub deafened: Option<bool>,
    pub talking: Option<bool>,
}

impl StateCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn channel(&self, id: u32) -> Option<&Channel> {
        self.channels.get(&id)
    }

    pub fn user(&self, id: u32) -> Option<&User> {
        self.users.get(&id)
    }

    pub fn apply_channel_state(&mut self, update: ChannelStateUpdate) {
        let entry = self.channels.entry(update.id).or_insert_with(|| Channel {
            id: update.id,
            name: String::from(""),
            parent_id: None,
        });

        if let Some(name) = update.name {
            entry.name = name;
        }

        if let Some(parent_id) = update.parent_id {
            entry.parent_id = Some(parent_id);
        }
    }

    pub fn apply_user_state(&mut self, update: UserStateUpdate) {
        let entry = self.users.entry(update.id).or_insert_with(|| User {
            id: update.id,
            name: String::from("Unknown"),
            channel_id: 0,
            muted: false,
            deafened: false,
            talking: false,
        });

        if let Some(name) = update.name {
            entry.name = name;
        }

        if let Some(channel_id) = update.channel_id {
            entry.channel_id = channel_id;
        }

        if let Some(muted) = update.muted {
            entry.muted = muted;
        }

        if let Some(deafened) = update.deafened {
            entry.deafened = deafened;
        }

        if let Some(talking) = update.talking {
            entry.talking = talking;
        }
    }

    pub fn apply_user_remove(&mut self, id: u32) {
        self.users.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::{ChannelStateUpdate, StateCache, UserStateUpdate};

    #[test]
    fn channel_state_adds_and_updates() {
        let mut cache = StateCache::new();

        cache.apply_channel_state(ChannelStateUpdate {
            id: 1,
            name: Some(String::from("Lobby")),
            parent_id: None,
        });

        let channel = cache.channel(1).expect("channel missing");
        assert_eq!(channel.name, "Lobby");
        assert_eq!(channel.parent_id, None);

        cache.apply_channel_state(ChannelStateUpdate {
            id: 1,
            name: Some(String::from("Main")),
            parent_id: Some(2),
        });

        let channel = cache.channel(1).expect("channel missing");
        assert_eq!(channel.name, "Main");
        assert_eq!(channel.parent_id, Some(2));
    }

    #[test]
    fn user_state_adds_and_updates() {
        let mut cache = StateCache::new();

        cache.apply_user_state(UserStateUpdate {
            id: 10,
            name: Some(String::from("Alice")),
            channel_id: Some(1),
            muted: Some(false),
            deafened: Some(false),
            talking: Some(false),
        });

        let user = cache.user(10).expect("user missing");
        assert_eq!(user.name, "Alice");
        assert_eq!(user.channel_id, 1);
        assert!(!user.muted);
        assert!(!user.deafened);

        cache.apply_user_state(UserStateUpdate {
            id: 10,
            name: None,
            channel_id: Some(2),
            muted: Some(true),
            deafened: None,
            talking: Some(true),
        });

        let user = cache.user(10).expect("user missing");
        assert_eq!(user.name, "Alice");
        assert_eq!(user.channel_id, 2);
        assert!(user.muted);
        assert!(!user.deafened);
        assert!(user.talking);
    }

    #[test]
    fn user_remove_deletes_user() {
        let mut cache = StateCache::new();

        cache.apply_user_state(UserStateUpdate {
            id: 11,
            name: Some(String::from("Eve")),
            channel_id: Some(1),
            muted: None,
            deafened: None,
            talking: None,
        });

        assert!(cache.user(11).is_some());

        cache.apply_user_remove(11);
        assert!(cache.user(11).is_none());
    }
}
