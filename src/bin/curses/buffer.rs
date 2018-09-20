use super::*;
use riirc::Queue;
use std::sync::RwLock;

pub struct Buffer {
    name: String,
    messages: RwLock<Queue<Output>>, // TODO use a Cow here
}

impl Buffer {
    pub fn new(name: impl AsRef<str>, max: usize) -> Self {
        let name = name.as_ref().to_owned();
        Self {
            name,
            messages: RwLock::new(Queue::new(max)),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn push_message(&self, msg: &Output) {
        trace!("{} <- {}", self.name(), msg.data);
        self.messages.write().unwrap().push(msg.clone());
    }

    pub fn most_recent(&self) -> Option<Output> {
        self.messages.read().unwrap().back().cloned()
    }

    pub fn clear(&self) {
        self.messages.write().unwrap().clear();
    }

    // this copies all of the messages
    pub fn messages(&self) -> Vec<Output> {
        let messages = &self.messages.read().unwrap();
        messages.iter().cloned().collect()
    }
}
