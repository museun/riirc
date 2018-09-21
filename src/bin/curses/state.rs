use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crossbeam_channel as channel;

use super::*;
use riirc::{Command, IrcClient, Prefix};

type ErrorChannel = channel::Receiver<riirc::IrcError>;

pub struct State {
    inner: RwLock<Inner>,
    queue: Arc<MessageQueue<Request>>,
}

impl MessageReceiver<Request> for State {
    fn queue(&self, data: impl Into<Request>) {
        self.queue.enqueue(data.into())
    }
}

struct Inner {
    client: Option<Arc<riirc::Client>>,
    errors: Option<Arc<ErrorChannel>>,
    buffers: VecDeque<Arc<Buffer>>,
    active_buffer: usize,

    // TODO this should hide the mutex
    config: Arc<RwLock<Config>>,
}

const BUFFER_MAX_SIZE: usize = 25 * 5;

// TODO split this up

impl State {
    pub fn new(queue: Arc<MessageQueue<Request>>, config: Arc<RwLock<Config>>) -> Self {
        let mut buffers = VecDeque::new();
        buffers.push_back(Arc::new(Buffer::new("*status", BUFFER_MAX_SIZE)));

        Self {
            inner: RwLock::new(Inner {
                client: None,
                errors: None,
                buffers,
                active_buffer: 0,

                config,
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
                let new = Arc::new(Buffer::new(name, BUFFER_MAX_SIZE));
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
        self.queue.enqueue(Request::Clear(false));
        for output in buffer.messages() {
            self.queue
                .enqueue(Request::Target(inner.active_buffer, output))
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

    pub fn send_line(&self, data: impl AsRef<str>) {
        if self.inner.read().unwrap().client.is_none() {
            let output = Output::new()
                .fg(Color::Red)
                .add("error: ")
                .add("not connected to server")
                .build();
            self.output(output, true);
            return;
        }

        let (index, buffer) = self.current_buffer();
        if buffer.name().starts_with('*') {
            let output = Output::new()
                .fg(Color::Red)
                .add("error: ")
                .add("cannot send to: ")
                .fg(Color::Cyan)
                .add(buffer.name())
                .build();
            self.output(output, true);
            return;
        }

        let data = data.as_ref();
        let nickname = {
            let client = self.client().unwrap();
            client.privmsg(&buffer.name(), data);

            client
                .state()
                .nickname()
                .expect("client should have a valid nickname")
        };

        let ts = timestamp(Instant::now());

        let mut output = Output::new();
        output.add(ts).add(" ");
        output.fg(Color::Green).add(nickname);
        let output = output.add(" ").add(data).build();

        self.queue.enqueue(Request::Queue(index, output));
    }

    pub fn set_client(&self, client: riirc::Client) {
        let inner = &mut self.inner.write().unwrap();
        let errors = client.errors();
        inner.client = Some(Arc::new(client));
        inner.errors = Some(Arc::new(errors));
    }

    pub fn client(&self) -> Option<Arc<riirc::Client>> {
        let inner = self.inner.read().unwrap();
        inner.client.as_ref().map(Arc::clone)
    }

    pub fn config(&self) -> Arc<RwLock<Config>> {
        let inner = self.inner.read().unwrap();
        Arc::clone(&inner.config)
    }

    pub fn read_errors(&self) -> Option<Arc<ErrorChannel>> {
        let inner = self.inner.read().unwrap();
        inner.errors.as_ref().map(Arc::clone)
    }

    // TODO remove this
    pub fn update(&self) -> Option<()> {
        let client = self.client()?;
        let (ts, msg) = client.state().next_message()?;
        let (current, active) = self.current_buffer();
        let me = client.state().nickname();

        match &msg.command {
            Command::Privmsg {
                target,
                data,
                is_notice,
            }
                if !is_notice =>
            {
                let target = match me {
                    Some(ref me) if me == target => {
                        Self::get_nick_for(&msg).or_else(|| Some(&target)).unwrap()
                    }
                    _ => target,
                };

                let (pos, buf) = match self.get_buffer_for_name(target) {
                    Some((pos, buf)) => (pos, buf),
                    None => self.new_buffer(target, false),
                };

                let nick = Self::get_nick_for(&msg)?;
                let ts = timestamp(ts);

                let mut output = Output::new();
                output.add(ts).add(" ");
                output.fg(Color::Red).add(nick);
                let output = output.add(" ").add(data).build();

                buf.push_message(&output);

                if current == pos as usize {
                    self.queue.enqueue(Request::Target(current, output));
                }
            }

            // Notices go to the status window
            Command::Privmsg { target, data, .. } => {
                let nick = Self::get_nick_for(&msg)?;

                let output = if me.is_none() || *target == me.unwrap() {
                    Output::new().add(format!("-{}- {}", nick, data)).build()
                } else {
                    Output::new()
                        .add(format!("-{} @ {}- {}", nick, target, data))
                        .build()
                };

                {
                    let inner = &mut self.inner.write().unwrap();
                    inner.buffers[0].push_message(&output);
                }
                if current == 0 {
                    self.queue.enqueue(Request::Target(current, output));
                }
            }

            Command::Join { channel, .. } => {
                trace!("join: {}", channel);
                let (pos, buf) = self.get_buffer_for_name(channel).or_else(|| {
                    trace!("cannot get buffer for: {}", channel);
                    None
                })?;
                let nick = Self::get_nick_for(&msg).or_else(|| {
                    trace!("cannot get nick for: {}", channel);
                    None
                })?;

                let mut output = Output::new();
                let output = if *nick == me.expect("self nick is required") {
                    output
                        .fg(Color::Green)
                        .add("joining")
                        .add(": ")
                        .add(channel)
                        .build()
                } else {
                    output
                        .fg(Color::Cyan)
                        .add(nick)
                        .fg(Color::Green)
                        .add(" joined ")
                        .add(channel)
                        .build()
                };

                // TODO abstract this
                self.queue.enqueue(Request::Queue(pos, output));
            }

            Command::Part {
                channel,
                ref reason,
            } => {
                let (pos, buf) = self.get_buffer_for_name(channel)?;
                let nick = Self::get_nick_for(&msg)?;
                if *nick != me.expect("self nick is required") {
                    let mut output = Output::new();
                    output
                        .fg(Color::Cyan)
                        .add(nick)
                        .fg(Color::Green)
                        .add(" left ")
                        .add(channel);

                    if let Some(reason) = reason {
                        output.add(": ").add(reason);
                    }

                    // TODO abstract this
                    self.queue.enqueue(Request::Queue(pos, output.build()));
                }
            }

            _ => {}
        }

        Some(())
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

impl Outputter for State {
    fn output(&self, output: impl Into<Output>, eol: bool) {
        self.queue.enqueue(Request::Queue(0, output.into()));
    }
}

// TODO use the timestamp from the client
fn timestamp(_instant: Instant) -> String {
    use chrono::prelude::*;
    let now: DateTime<Local> = Local::now();

    format!("{:02}{:02}{:02}", now.hour(), now.minute(), now.second())
}
