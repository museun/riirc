use super::*;

use crossbeam_channel as channel;

#[derive(Debug, PartialEq)]
pub enum Request {
    Clear(bool),
    Join(String),
    Part(String),
    Quit(Option<String>),

    ToggleNickList,
    ClearHistory(usize),

    SwitchBuffer(usize),
    NextBuffer,
    PrevBuffer,

    Queue(usize, Output),  // buffer index
    Target(usize, Output), // buffer index
}

pub struct MessageQueue {
    queue: channel::Sender<Request>,
    reader: channel::Receiver<Request>,
}

impl MessageQueue {
    pub fn new() -> Self {
        let (queue, reader) = channel::unbounded();
        Self { queue, reader }
    }

    pub fn request(&self, req: Request) {
        trace!("pushing: {:?}", req);
        self.queue.send(req);
    }

    // TODO impl Into<Output>
    pub fn queue(&self, buf: usize, output: Output) {
        self.request(Request::Queue(buf, output));
    }

    pub fn status(&self, output: Output) {
        self.queue(0, output);
    }

    pub fn read_queue(&self) -> Vec<Request> {
        let mut buf = Vec::with_capacity(self.reader.len());
        while let Some(req) = self.reader.try_recv() {
            buf.push(req)
        }
        buf
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn clear(&self) {
        while let Some(_req) = self.reader.try_recv() {}
    }
}
