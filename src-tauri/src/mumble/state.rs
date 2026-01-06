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

    pub fn channels(&self) -> Vec<Channel> {
        let mut channels = self.channels.values().cloned().collect::<Vec<_>>();
        channels.sort_by_key(|channel| channel.id);
        channels
    }

    pub fn users(&self) -> Vec<User> {
        let mut users = self.users.values().cloned().collect::<Vec<_>>();
        users.sort_by_key(|user| user.id);
        users
    }
}

#[cfg(test)]
mod tests {
    use super::{ChannelStateUpdate, StateCache, UserStateUpdate};

    /// Channel updates create and then update cached channel data.
    #[test]
    fn channel_state_adds_and_updates() {
        // Arrange
        let mut cache = StateCache::new();

        // Act
        cache.apply_channel_state(ChannelStateUpdate {
            id: 1,
            name: Some(String::from("Lobby")),
            parent_id: None,
        });

        // Assert
        let channel = cache.channel(1).expect("channel missing");
        assert_eq!(channel.name, "Lobby");
        assert_eq!(channel.parent_id, None);

        // Act
        cache.apply_channel_state(ChannelStateUpdate {
            id: 1,
            name: Some(String::from("Main")),
            parent_id: Some(2),
        });

        // Assert
        let channel = cache.channel(1).expect("channel missing");
        assert_eq!(channel.name, "Main");
        assert_eq!(channel.parent_id, Some(2));
    }

    /// User updates populate fields and apply subsequent changes.
    #[test]
    fn user_state_adds_and_updates() {
        // Arrange
        let mut cache = StateCache::new();

        // Act
        cache.apply_user_state(UserStateUpdate {
            id: 10,
            name: Some(String::from("Alice")),
            channel_id: Some(1),
            muted: Some(false),
            deafened: Some(false),
            talking: Some(false),
        });

        // Assert
        let user = cache.user(10).expect("user missing");
        assert_eq!(user.name, "Alice");
        assert_eq!(user.channel_id, 1);
        assert!(!user.muted);
        assert!(!user.deafened);

        // Act
        cache.apply_user_state(UserStateUpdate {
            id: 10,
            name: None,
            channel_id: Some(2),
            muted: Some(true),
            deafened: None,
            talking: Some(true),
        });

        // Assert
        let user = cache.user(10).expect("user missing");
        assert_eq!(user.name, "Alice");
        assert_eq!(user.channel_id, 2);
        assert!(user.muted);
        assert!(!user.deafened);
        assert!(user.talking);
    }

    /// Missing flags do not overwrite prior values when updates are partial.
    #[test]
    fn user_state_ignores_missing_flags() {
        // Arrange
        let mut cache = StateCache::new();

        // Act
        cache.apply_user_state(UserStateUpdate {
            id: 12,
            name: Some(String::from("Pat")),
            channel_id: Some(1),
            muted: Some(false),
            deafened: Some(true),
            talking: Some(false),
        });

        cache.apply_user_state(UserStateUpdate {
            id: 12,
            name: None,
            channel_id: None,
            muted: None,
            deafened: None,
            talking: None,
        });

        // Assert
        let user = cache.user(12).expect("user missing");
        assert!(!user.muted);
        assert!(user.deafened);
        assert!(!user.talking);
    }

    /// Removing a user deletes the cached entry.
    #[test]
    fn user_remove_deletes_user() {
        // Arrange
        let mut cache = StateCache::new();

        // Act
        cache.apply_user_state(UserStateUpdate {
            id: 11,
            name: Some(String::from("Eve")),
            channel_id: Some(1),
            muted: None,
            deafened: None,
            talking: None,
        });

        // Assert
        assert!(cache.user(11).is_some());

        // Act
        cache.apply_user_remove(11);
        // Assert
        assert!(cache.user(11).is_none());
    }

    /// Channel and user snapshots are sorted by identifier.
    #[test]
    fn channels_and_users_return_sorted_snapshots() {
        // Arrange
        let mut cache = StateCache::new();

        // Act
        cache.apply_channel_state(ChannelStateUpdate {
            id: 2,
            name: Some(String::from("Second")),
            parent_id: None,
        });
        cache.apply_channel_state(ChannelStateUpdate {
            id: 1,
            name: Some(String::from("First")),
            parent_id: None,
        });

        cache.apply_user_state(UserStateUpdate {
            id: 20,
            name: Some(String::from("Zed")),
            channel_id: Some(1),
            muted: Some(false),
            deafened: Some(false),
            talking: Some(false),
        });
        cache.apply_user_state(UserStateUpdate {
            id: 10,
            name: Some(String::from("Ann")),
            channel_id: Some(1),
            muted: Some(false),
            deafened: Some(false),
            talking: Some(false),
        });

        // Assert
        let channels = cache.channels();
        assert_eq!(channels[0].id, 1);
        assert_eq!(channels[1].id, 2);

        let users = cache.users();
        assert_eq!(users[0].id, 10);
        assert_eq!(users[1].id, 20);
    }
}
