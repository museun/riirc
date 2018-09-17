use std::sync::RwLock;

use riirc::Queue;

// TODO impl Eq for this
pub struct Buffer {
    name: String,
    messages: RwLock<Queue<String>>, // TODO use a Cow here
}

impl Buffer {
    pub fn new(name: impl AsRef<str>) -> Self {
        let name = name.as_ref().to_owned();
        Self {
            name,
            messages: RwLock::new(Queue::new(125)),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn push_message(&self, msg: &str) {
        trace!("{} <- {}", self.name(), msg);
        self.messages.write().unwrap().push(msg.into());
    }

    pub fn most_recent(&self) -> Option<String> {
        self.messages.read().unwrap().back().cloned()
    }

    pub fn clear(&self) {
        self.messages.write().unwrap().clear();
    }

    // this copies all of the messages
    pub fn messages(&self) -> Vec<String> {
        let messages = &self.messages.read().unwrap();
        messages.iter().map(|s| s.to_string()).collect()
    }
}
