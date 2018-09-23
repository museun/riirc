use super::buffer::Buffers;
use super::request::Request;
use super::*;

use crossbeam_channel as channel;
use std::rc::Rc;
use std::sync::RwLock;

type ErrorChannel = channel::Receiver<irc::Error>;

struct Inner {
    client: Option<Rc<irc::Client>>,
    errors: Option<Rc<ErrorChannel>>,
}

pub struct State {
    inner: RwLock<Inner>,
    queue: Rc<MessageQueue<Request>>,
    config: Rc<Config>,
    buffers: Rc<Buffers>,
}

impl State {
    pub fn new(queue: Rc<MessageQueue<Request>>, config: Rc<Config>) -> Self {
        Self {
            inner: RwLock::new(Inner {
                client: None,
                errors: None,
            }),
            buffers: Rc::new(Buffers::new(Rc::clone(&queue))),
            config,
            queue,
        }
    }

    pub fn reset(&self) {
        warn!("implement reset")
    }

    pub fn buffers(&self) -> Rc<Buffers> {
        Rc::clone(&self.buffers)
    }

    pub fn config(&self) -> Rc<Config> {
        Rc::clone(&self.config)
    }

    pub fn client(&self) -> Option<Rc<irc::Client>> {
        let inner = self.inner.read().unwrap();
        inner.client.as_ref().map(Rc::clone)
    }

    pub fn set_client(&self, client: irc::Client) {
        let inner = &mut self.inner.write().unwrap();
        let errors = client.errors();
        inner.client = Some(Rc::new(client));
        inner.errors = Some(Rc::new(errors));
    }

    pub fn queue(&self, req: Request) {
        self.queue.enqueue(req);
    }

    pub fn read_errors(&self) -> Option<Rc<ErrorChannel>> {
        let inner = self.inner.read().unwrap();
        inner.errors.as_ref().map(Rc::clone)
    }

    pub fn read_requests(&self) -> Vec<Request> {
        self.queue.read_all()
    }
}
