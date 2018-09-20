use super::*;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

const MAX_BACKLOG: usize = 512;

// TODO don't expose this
pub(crate) struct State {
    pub(crate) nickname: Option<String>,
    pub(crate) channels: HashMap<String, Arc<Channel>>,
    pub(crate) backlog: Queue<(Instant, Message)>,
}

impl State {
    pub fn new() -> Self {
        Self {
            nickname: None,
            channels: HashMap::new(),
            backlog: Queue::new(MAX_BACKLOG),
        }
    }

    pub fn is_from_self(&self, msg: &Message) -> bool {
        if let Some(Prefix::User { nick, .. }) = &msg.prefix {
            if let Some(current) = &self.nickname {
                return current == nick;
            }
        }
        false
    }

    pub fn new_channel(&mut self, channel: &str) {
        // TODO this should never happen
        if self.channels.contains_key(channel) {
            warn!("joined existing channel: {}", channel);
            return;
        }

        let chan = Arc::new(Channel::new());
        self.channels.insert(channel.to_owned(), chan);
    }

    pub fn nick_join(&self, channel: &str, nick: &str) {
        // TODO this should never happen
        if !self.channels.contains_key(channel) {
            warn!("user: {} joined a missing channel: {}", nick, channel);
            return;
        }

        self.channels[channel].add_user(nick)
    }

    pub fn remove_channel(&mut self, channel: &str) {
        // TODO this should never happen
        if !self.channels.contains_key(channel) {
            warn!("left a missing channel: {}", channel);
            return;
        }

        self.channels.remove(channel);
    }

    pub fn nick_part(&self, channel: &str, nick: &str) {
        // TODO this should never happen
        if !self.channels.contains_key(channel) {
            warn!("user: {} left a missing channel: {}", nick, channel);
            return;
        }

        self.channels[channel].remove_user(&nick)
    }

    pub fn update_nick(&self, old: &str, nick: &str) {
        for channel in self.channels.values() {
            // this isn't needed. but it makes the logic apparent
            if channel.has_user(&old) {
                channel.update_user(&old, &nick)
            }
        }
    }

    pub fn remove_nick(&self, nick: &str) {
        for channel in self.channels.values() {
            channel.remove_user(&nick)
        }
    }
}
