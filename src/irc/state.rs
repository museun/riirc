use super::*;

use std::sync::{Arc, RwLock};
use std::time::Instant;

const MAX_BACKLOG: usize = 512;

struct Inner {
    nickname: Option<String>,
    backlog: Queue<(Instant, Message)>,
}

pub struct State {
    channels: Arc<Channels>,
    inner: RwLock<Inner>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            inner: RwLock::new(Inner {
                nickname: None,
                backlog: Queue::new(MAX_BACKLOG),
            }),
            channels: Arc::new(Channels::new()),
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn channels(&self) -> Arc<Channels> {
        Arc::clone(&self.channels)
    }

    pub fn nickname(&self) -> Option<String> {
        self.inner.read().unwrap().nickname.clone()
    }

    pub(crate) fn set_nickname(&self, nick: impl Into<String>) {
        self.inner.write().unwrap().nickname = Some(nick.into());
    }

    pub fn push_message(&self, msg: (Instant, Message)) {
        self.inner.write().unwrap().backlog.push(msg);
    }

    pub fn next_message(&self) -> Option<(Instant, Message)> {
        self.inner.write().unwrap().backlog.pop()
    }
}
