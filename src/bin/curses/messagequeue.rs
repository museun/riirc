use crossbeam_channel as channel;

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

pub struct MessageQueue {
    queue: channel::Sender<Request>,
    reader: channel::Receiver<Request>,
}

// TODO add logging here
impl MessageQueue {
    pub fn new() -> Self {
        let (queue, reader) = channel::bounded(16);
        Self { queue, reader }
    }

    pub fn push(&self, req: Request) {
        trace!("pushing: {:?}", req);
        self.queue.send(req);
    }

    pub fn queue(&self, buf: usize, data: impl AsRef<str>) {
        self.push(Request::Queue(buf, data.as_ref().to_owned()));
    }

    pub fn status(&self, data: impl AsRef<str>) {
        self.queue(0, data);
    }

    pub fn read_queue(&self) -> Vec<Request> {
        let mut buf = vec![]; // TODO reader.len() as cap
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
