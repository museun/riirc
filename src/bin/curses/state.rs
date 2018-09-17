use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};

use super::*;
use riirc::{Command, IrcClient, Prefix};

pub struct State {
    inner: RwLock<Inner>,
    queue: Arc<MessageQueue>,
}

struct Inner {
    client: Option<Arc<riirc::Client>>,
    errors: Option<Arc<Receiver<riirc::Error>>>,
    buffers: VecDeque<Arc<Buffer>>,
    active_buffer: usize,
}

impl State {
    pub fn new(queue: Arc<MessageQueue>) -> Self {
        let mut buffers = VecDeque::new();
        buffers.push_back(Arc::new(Buffer::new("*status")));

        Self {
            inner: RwLock::new(Inner {
                client: None,
                errors: None,
                buffers,
                active_buffer: 0,
            }),
            queue: queue,
        }
    }

    // TODO make this more fine-grained
    pub fn reset(&self) {
        let inner = &mut self.inner.write().unwrap();
        let status = inner.buffers.pop_front();
        inner.buffers.clear();
        inner.buffers.push_back(status.unwrap());

        self.queue.clear();

        let client = inner.client.take().expect("client to exist");
        let errors = inner.errors.take().expect("errors channel to exist");

        debug_assert_eq!(Arc::strong_count(&client), 1);
        debug_assert_eq!(Arc::strong_count(&errors), 1);
    }

    pub fn new_buffer(&self, name: impl AsRef<str>, activate: bool) -> (usize, Arc<Buffer>) {
        let name = name.as_ref();
        trace!("creating new buffer: {}", name);
        let (pos, buf) = {
            let inner = &mut self.inner.write().unwrap();
            let pos = if let Some(pos) = inner.buffers.iter().position(|b| b.name() == name) {
                if activate {
                    trace!("already exists, forcing activation");
                    inner.active_buffer = pos;
                }
                pos
            } else {
                let new = Arc::new(Buffer::new(name));
                inner.buffers.push_back(new);
                if activate {
                    trace!("created, forcing activation");
                    inner.active_buffer = inner.buffers.len() - 1;
                }
                inner.buffers.len() - 1
            };
            (pos, Arc::clone(&inner.buffers[pos]))
        };
        if activate {
            self.display_buffer();
        }
        trace!("new_buffer: {}, {}", pos, buf.name());
        (pos, buf)
    }

    pub fn remove_buffer(&self, name: impl AsRef<str>) {
        {
            let inner = &mut self.inner.write().unwrap();
            let name = name.as_ref();
            if let Some(pos) = inner.buffers.iter().position(|b| b.name() == name) {
                inner.buffers.remove(pos);
                inner.active_buffer = 0;
            } else {
                warn!("not a buffer: {}", name);
                return;
            }
        }
        self.display_buffer();
    }

    pub fn current_buffer(&self) -> (usize, Arc<Buffer>) {
        let inner = self.inner.read().unwrap();
        let active = inner.active_buffer;
        (active, Arc::clone(&inner.buffers[active]))
    }

    pub fn activate_buffer(&self, buf: usize) {
        {
            let inner = &mut self.inner.write().unwrap();
            // sanity check
            if buf as usize >= inner.buffers.len() {
                return;
            }
            inner.active_buffer = buf as usize;
        }
        trace!("activating buffer: {}", buf);
        self.display_buffer();
    }

    pub fn activate_buffer_by_name(&self, name: impl AsRef<str>) {
        let name = name.as_ref();
        {
            let inner = &mut self.inner.write().unwrap();
            if let Some(pos) = inner.buffers.iter().position(|b| b.name() == name) {
                inner.active_buffer = pos;
            }
        }

        self.display_buffer();
    }

    pub fn display_buffer(&self) {
        let inner = self.inner.read().unwrap();
        let buffer = Arc::clone(&inner.buffers[inner.active_buffer]);

        let name = buffer.name().to_string();
        self.queue.push(Request::Clear(false));
        for msg in buffer.messages() {
            self.queue
                .push(Request::Target(inner.active_buffer, msg.to_string()))
        }
    }

    pub fn at_status_buffer(&self) -> (bool, Arc<Buffer>) {
        let inner = &mut self.inner.write().unwrap();
        let active = inner.active_buffer;
        let buffers = &mut inner.buffers;

        let buf = Arc::clone(&buffers[active]);
        (buf.name().starts_with('*'), buf)
    }

    pub fn get_buffer(&self, index: usize) -> Option<Arc<Buffer>> {
        let inner = &*self.inner.write().unwrap();
        inner.buffers.get(index).map(Arc::clone)
    }

    pub fn get_buffer_for_name(&self, name: impl AsRef<str>) -> Option<(usize, Arc<Buffer>)> {
        let name = name.as_ref();
        let buffers = &self.inner.read().unwrap().buffers;
        let pos = buffers.iter().position(|b| b.name() == name);
        pos.and_then(|p| buffers.get(p).map(Arc::clone).and_then(|b| Some((p, b))))
    }

    pub fn buffers(&self) -> Vec<Arc<Buffer>> {
        let buffers = &self.inner.read().unwrap().buffers;
        buffers.iter().map(Arc::clone).collect()
    }

    pub fn buffers_len(&self) -> usize {
        let buffers = &self.inner.read().unwrap().buffers;
        buffers.len()
    }

    pub fn send_message(&self, data: impl AsRef<str>) {
        if self.inner.read().unwrap().client.is_none() {
            self.queue.status("not connected to a server");
            return;
        }

        let (index, buffer) = { self.current_buffer() };
        if buffer.name().starts_with('*') {
            self.queue
                .status(format!("cannot send to {}", buffer.name()));
            return;
        }

        let data = data.as_ref();
        let nickname = {
            let client = self.client().unwrap();
            client.privmsg(&buffer.name(), data);
            client
                .nickname()
                .expect("client should have a valid nickname")
        };

        let msg = format!("{}: {}", nickname, data);
        self.queue.push(Request::Queue(index, msg));
    }

    pub fn assume_connected(&self) -> bool {
        let inner = &self.inner.read().unwrap();
        if inner.client.is_none() {
            self.queue.status("not connected");
            return false;
        }
        true
    }

    pub fn set_client(&self, client: riirc::Client, errors: Receiver<riirc::Error>) {
        let inner = &mut self.inner.write().unwrap();
        inner.client = Some(Arc::new(client));
        inner.errors = Some(Arc::new(errors));
    }

    pub fn client(&self) -> Option<Arc<riirc::Client>> {
        let inner = self.inner.read().unwrap();
        inner.client.as_ref().map(Arc::clone)
    }

    pub fn read_errors(&self) -> Option<Arc<Receiver<riirc::Error>>> {
        let inner = self.inner.read().unwrap();
        inner.errors.as_ref().map(Arc::clone)
    }

    pub fn sync_state(&self) {
        if self.client().is_none() {
            return;
        }

        let (ts, msg) = {
            let inner = &mut self.inner.write().unwrap();
            let res = inner
                .client
                .as_ref()
                .map(Arc::clone)
                .and_then(|c| c.next_message());
            if res.is_none() {
                return;
            }
            res.unwrap()
        };

        let (current, active) = self.current_buffer();
        let me = {
            let client = self.client().unwrap();
            client.nickname()
        };

        match &msg.command {
            Command::Privmsg {
                target,
                data,
                is_notice,
            }
                if !is_notice =>
            {
                let target = if me.is_some() && me.as_ref().unwrap() == target {
                    Self::get_nick_for(&msg).or_else(|| Some(target)).unwrap()
                } else {
                    target
                };

                let (pos, buf) = match self.get_buffer_for_name(target) {
                    Some((pos, buf)) => (pos, buf),
                    None => self.new_buffer(target, false),
                };

                // TODO add timestamp
                if let Some(nick) = Self::get_nick_for(&msg) {
                    let data = format!("{}: {}", nick, data);
                    buf.push_message(&data);

                    if current == pos as usize {
                        self.queue.push(Request::Target(current, data))
                    }
                }
            }

            // Notices go to the status window
            Command::Privmsg { target, data, .. } => {
                let inner = &mut self.inner.write().unwrap();
                let buf = Arc::clone(&inner.buffers[0]);

                // TODO add timestamp
                if let Some(nick) = Self::get_nick_for(&msg) {
                    let data = if me.is_none() || target == me.as_ref().unwrap() {
                        format!("-{}- {}", nick, data)
                    } else {
                        format!("-{} @ {}- {}", nick, target, data)
                    };

                    buf.push_message(&data);
                    if current == 0 {
                        self.queue.push(Request::Target(current, data))
                    }
                }
            }

            Command::Join { target, .. } => {
                let target = &target[0];

                let (pos, buf) = self
                    .get_buffer_for_name(target)
                    .expect("buffer should have been created");

                let nick = Self::get_nick_for(&msg).expect("join requires a name");
                if *nick == me.expect("self nick is required") {
                    self.queue
                        .push(Request::Queue(pos, format!("Joining: {}", target)))
                }
            }
            Command::Part { target, reason } => {}

            _ => {}
        }

        // TODO synchronize the users
    }

    fn get_nick_for(msg: &riirc::Message) -> Option<&str> {
        match &msg.prefix {
            Some(Prefix::User { nick, .. }) => Some(&nick),
            Some(Prefix::Server { host }) => Some(&host),
            _ => None,
        }
    }
}
