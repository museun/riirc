use crossbeam_channel as channel;

pub trait MessageReceiver<T> {
    fn queue(&self, data: impl Into<T>);
}

pub struct MessageQueue<T> {
    queue: channel::Sender<T>,
    reader: channel::Receiver<T>,
}

impl<T> MessageQueue<T> {
    pub fn new() -> Self {
        let (queue, reader) = channel::unbounded();
        Self { queue, reader }
    }

    pub fn enqueue(&self, req: T) {
        self.queue.send(req);
    }

    pub fn read_all(&self) -> Vec<T> {
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
