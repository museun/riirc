use std::collections::HashSet;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Channel {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    topic: Option<Arc<String>>,
    users: HashSet<String>,
}

impl Channel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_topic(&self, topic: impl AsRef<str>) {
        let inner = &mut self.inner.write().unwrap();
        inner.topic = Some(Arc::new(topic.as_ref().to_owned()))
    }

    pub fn topic(&self) -> Option<Arc<String>> {
        let inner = &self.inner.read().unwrap();
        inner.topic.as_ref().map(Arc::clone)
    }

    pub fn has_user(&self, nick: &str) -> bool {
        let inner = &self.inner.read().unwrap();
        inner.users.contains(nick)
    }

    pub fn add_user(&self, nick: &str) {
        let inner = &mut self.inner.write().unwrap();
        inner.users.insert(nick.into());
    }

    pub fn update_user(&self, old: &str, new: &str) {
        let inner = &mut self.inner.write().unwrap();
        if inner.users.contains(old) {
            inner.users.remove(old);
            inner.users.insert(new.into());
        }
    }

    pub fn remove_user(&self, user: &str) {
        let inner = &mut self.inner.write().unwrap();
        inner.users.remove(user);
    }

    // TODO get rid of this clone
    pub fn users(&self) -> Vec<String> {
        let inner = &*self.inner.read().unwrap();
        inner.users.iter().cloned().collect()
    }
}
