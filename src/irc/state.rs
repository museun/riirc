use super::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

const MAX_BACKLOG: usize = 512;

struct Inner {
    nickname: Option<String>,
    // TODO use a tagging system instead of this tree approach
    channels: HashMap<String, Arc<Channel>>,
    backlog: Queue<(Instant, Message)>,
}

pub struct State {
    inner: RwLock<Inner>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            inner: RwLock::new(Inner {
                nickname: None,
                channels: HashMap::new(),
                backlog: Queue::new(MAX_BACKLOG),
            }),
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_message(&self, msg: (Instant, Message)) {
        self.inner.write().unwrap().backlog.push(msg);
    }

    pub fn new_channel(&self, channel: &str) {
        if self.inner.read().unwrap().channels.contains_key(channel) {
            warn!("joined existing channel: {}", channel);
            return;
        }

        let chan = Arc::new(Channel::new());
        self.inner
            .write()
            .unwrap()
            .channels
            .insert(channel.to_owned(), chan);
    }

    pub fn remove_channel(&self, channel: &str) {
        if !self.inner.read().unwrap().channels.contains_key(channel) {
            warn!("left a missing channel: {}", channel);
            return;
        }
        self.inner.write().unwrap().channels.remove(channel);
    }

    pub fn has_channel(&self, channel: &str) -> bool {
        self.inner.read().unwrap().channels.contains_key(channel)
    }

    pub fn get_channel(&self, channel: &str) -> Option<Arc<Channel>> {
        self.inner
            .read()
            .unwrap()
            .channels
            .get(channel)
            .map(Arc::clone)
    }

    pub fn nick_join(&self, channel: &str, nick: &str) {
        if !self.has_channel(channel) {
            return;
        }

        self.inner.write().unwrap().channels[channel].add_user(nick)
    }

    pub fn nick_part(&self, channel: &str, nick: &str) {
        if !self.has_channel(channel) {
            return;
        }
        self.inner.write().unwrap().channels[channel].remove_user(&nick)
    }

    pub fn update_nick(&self, old: &str, nick: &str) {
        for channel in self.inner.write().unwrap().channels.values() {
            // this isn't needed. but it makes the logic apparent
            if channel.has_user(&old) {
                channel.update_user(&old, &nick)
            }
        }
    }

    pub fn remove_nick(&self, nick: &str) {
        for channel in self.inner.write().unwrap().channels.values() {
            channel.remove_user(&nick)
        }
    }

    pub fn nickname(&self) -> Option<String> {
        self.inner.read().unwrap().nickname.clone()
    }

    pub(crate) fn set_nickname(&self, nick: &str) {
        self.inner.write().unwrap().nickname = Some(nick.to_owned());
    }

    pub fn is_from_self(&self, msg: &Message) -> bool {
        if let Some(Prefix::User { nick, .. }) = &msg.prefix {
            if let Some(current) = &self.inner.read().unwrap().nickname {
                return current == nick;
            }
        }
        false
    }

    pub fn next_message(&self) -> Option<(Instant, Message)> {
        let inner = &mut *self.inner.write().unwrap();
        inner.backlog.pop()
    }
}
