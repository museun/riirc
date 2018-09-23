use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Channels {
    data: RwLock<HashMap<String, Arc<Channel>>>,
}

impl Channels {
    pub fn new() -> Self {
        Channels::default()
    }

    pub fn add(&self, channel: impl Into<String>) -> Arc<Channel> {
        let channel = channel.into();

        debug_assert!(
            self.contains(&channel),
            "channel '{}' shouldn't exist",
            &channel
        );

        let chan = Arc::new(Channel::new());
        self.data
            .write()
            .unwrap()
            .insert(channel, Arc::clone(&chan));
        chan
    }

    pub fn remove(&self, channel: impl AsRef<str>) {
        let channel = channel.as_ref();

        debug_assert!(
            !self.contains(&channel),
            "channel '{}' should exist",
            &channel
        );

        self.data.write().unwrap().remove(channel);
    }

    pub fn contains(&self, channel: impl AsRef<str>) -> bool {
        self.data.read().unwrap().contains_key(channel.as_ref())
    }

    pub fn get(&self, channel: impl AsRef<str>) -> Option<Arc<Channel>> {
        self.data
            .read()
            .unwrap()
            .get(channel.as_ref())
            .map(Arc::clone)
    }

    pub fn update_nick(&self, old: impl AsRef<str>, nick: impl Into<String>) {
        let old = old.as_ref();
        let nick = nick.into();

        for ch in self.data.write().unwrap().values() {
            ch.update(old, nick.clone())
        }
    }

    pub fn clear_nick(&self, nick: impl AsRef<str>) {
        for ch in self.data.write().unwrap().values() {
            ch.remove(&nick)
        }
    }
}

// FIXME: this does all sorts of dumb allocations
#[derive(Default)]
pub struct Channel {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    topic: Option<String>,
    users: HashSet<Arc<String>>,
}

impl Channel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_topic(&self, topic: impl Into<String>) {
        let inner = &mut self.inner.write().unwrap();
        inner.topic = Some(topic.into())
    }

    pub fn topic(&self) -> Option<String> {
        let inner = &self.inner.read().unwrap();
        inner.topic.clone()
    }

    pub fn contains(&self, nick: impl AsRef<str>) -> bool {
        let inner = &self.inner.read().unwrap();
        inner.users.contains(&nick.as_ref().to_owned())
    }

    pub fn add(&self, nick: impl Into<String>) {
        let inner = &mut self.inner.write().unwrap();
        inner.users.insert(Arc::new(nick.into()));
    }

    pub fn remove(&self, user: impl AsRef<str>) {
        let inner = &mut self.inner.write().unwrap();
        inner.users.remove(&user.as_ref().to_owned());
    }

    pub fn update(&self, old: impl AsRef<str>, new: impl Into<String>) {
        let old = old.as_ref().to_owned();
        let inner = &mut self.inner.write().unwrap();
        if inner.users.contains(&old) {
            inner.users.remove(&old);
            inner.users.insert(Arc::new(new.into()));
        }
    }

    pub fn users(&self) -> Vec<Arc<String>> {
        let inner = &*self.inner.read().unwrap();
        let mut v = inner.users.iter().map(Arc::clone).collect::<Vec<_>>();
        // IRC uses a non ASCII lexigraphical comparison for nicknames
        // TODO implement it
        v.sort();
        v
    }
}
