use std::collections::VecDeque;
use std::sync::RwLock;

#[derive(Debug, PartialEq)]
pub enum Request {
    Clear(bool),
    Join(String),
    Part(String),
    Quit(Option<String>),

    SwitchBuffer(usize),
    NextBuffer,
    PrevBuffer,

    Queue(usize, String),  // buffer index
    Target(usize, String), // buffer index
}

#[derive(Default)]
pub struct MessageQueue {
    queue: RwLock<VecDeque<Request>>,
}

// TODO add logging here
impl MessageQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, req: Request) {
        self.queue.write().unwrap().push_back(req)
    }

    pub fn status(&self, data: impl AsRef<str>) {
        self.push(Request::Queue(0, data.as_ref().to_owned()))
    }

    pub fn read_queue(&self) -> Vec<Request> {
        self.queue.write().unwrap().drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.queue.read().unwrap().len()
    }

    pub fn clear(&self) {
        self.queue.write().unwrap().clear();
    }
}
