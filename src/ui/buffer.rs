use super::output::Output;
use super::request::Request;
use super::*;

use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::RwLock;

const BUFFER_MAX_SIZE: usize = 25 * 5;

struct Inner {
    buffers: VecDeque<Rc<Buffer>>,
    active: usize,
}

pub struct Buffers {
    inner: RwLock<Inner>,
    queue: Rc<MessageQueue<Request>>,
}

impl Buffers {
    pub fn new(queue: Rc<MessageQueue<Request>>) -> Self {
        let mut buffers = VecDeque::new();
        buffers.push_back(Rc::new(Buffer::new("*status", BUFFER_MAX_SIZE)));
        Self {
            inner: RwLock::new(Inner { buffers, active: 0 }),
            queue,
        }
    }

    pub fn create(&self, name: impl AsRef<str>, activate: bool) -> (usize, Rc<Buffer>) {
        let name = name.as_ref();
        trace!("creating new buffer: {}", name);
        let (pos, buf) = {
            let inner = &mut self.inner.write().unwrap();
            let pos = if let Some(pos) = inner.buffers.iter().position(|b| b.name() == name) {
                if activate {
                    trace!("already exists, forcing activation");
                    inner.active = pos;
                }
                pos
            } else {
                let new = Rc::new(Buffer::new(name, BUFFER_MAX_SIZE));
                inner.buffers.push_back(new);
                if activate {
                    trace!("created, forcing activation");
                    inner.active = inner.buffers.len() - 1;
                }
                inner.buffers.len() - 1
            };
            (pos, Rc::clone(&inner.buffers[pos]))
        };
        if activate {
            self.display();
        }
        trace!("new_buffer: {}, {}", pos, buf.name());
        (pos, buf)
    }

    pub fn delete(&self, name: impl AsRef<str>) {
        {
            let inner = &mut self.inner.write().unwrap();
            let name = name.as_ref();
            if let Some(pos) = inner.buffers.iter().position(|b| b.name() == name) {
                inner.buffers.remove(pos);
                inner.active = 0;
            } else {
                warn!("not a buffer: {}", name);
                return;
            }
        }
        self.display();
    }

    pub fn activate(&self, buf: usize) {
        if buf > self.buffers().len() {
            return;
        }

        {
            let inner = &mut self.inner.write().unwrap();
            inner.active = buf;
        }

        trace!("activating buffer: {}", buf);
        self.display()
    }

    pub fn activate_by_name(&self, name: impl AsRef<str>) {
        let name = name.as_ref();
        {
            let inner = &mut self.inner.write().unwrap();
            if let Some(pos) = inner.buffers.iter().position(|b| b.name() == name) {
                inner.active = pos;
            }
        }

        trace!("activating buffer: {}", name);
        self.display()
    }

    pub fn current(&self) -> (usize, Rc<Buffer>) {
        let inner = &*self.inner.read().unwrap();
        (inner.active, Rc::clone(&inner.buffers[inner.active]))
    }

    pub fn buffers(&self) -> Vec<Rc<Buffer>> {
        let inner = &*self.inner.read().unwrap();
        inner.buffers.iter().map(Rc::clone).collect()
    }

    pub fn len(&self) -> usize {
        let inner = &*self.inner.read().unwrap();
        inner.buffers.len()
    }

    /// will always have atleast 1 buffer
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn get(&self, index: usize) -> Option<Rc<Buffer>> {
        let buffers = &self.inner.read().unwrap().buffers;
        buffers.get(index).map(Rc::clone)
    }

    pub fn index_of(&self, name: impl AsRef<str>) -> Option<usize> {
        let name = name.as_ref();
        let buffers = &self.inner.read().unwrap().buffers;
        buffers.iter().position(|b| b.name() == name)
    }

    pub fn named(&self, name: impl AsRef<str>) -> Option<(usize, Rc<Buffer>)> {
        let name = name.as_ref();
        let buffers = &self.inner.read().unwrap().buffers;
        let pos = buffers.iter().position(|b| b.name() == name);
        pos.and_then(|p| buffers.get(p).map(Rc::clone).and_then(|b| Some((p, b))))
    }

    fn display(&self) {
        let (index, buffer) = self.current();
        self.queue.enqueue(Request::Clear(false));
        for output in buffer.messages() {
            self.queue.enqueue(Request::Target(index, output));
        }
    }
}

pub struct Buffer {
    name: String,
    messages: RwLock<irc::Queue<Output>>,
}

impl Buffer {
    pub fn new(name: impl Into<String>, max: usize) -> Self {
        Self {
            name: name.into(),
            messages: RwLock::new(irc::Queue::new(max)),
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

    /// this copies all of the messages
    pub fn messages(&self) -> Vec<Output> {
        let messages = &self.messages.read().unwrap();
        messages.iter().cloned().collect()
    }

    pub fn is_status(&self) -> bool {
        self.name.starts_with('*')
    }
}
