use super::ircclient::IrcClient;
use super::message::Error as MessageError;
use super::*;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Error as IoError};
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Instant;

const MAX_BACKLOG: usize = 512;

#[derive(Debug)]
pub enum Error {
    CannotConnect(IoError),
    ParseError(MessageError),
    CannotRead(IoError),
    EndOfStream,
}

// TODO don't expose this
pub(crate) struct State {
    pub(crate) nickname: Option<String>,
    pub(crate) channels: HashMap<String, Arc<Channel>>,
    pub(crate) backlog: Queue<(Instant, Message)>,
}

impl State {
    pub fn new() -> Self {
        Self {
            nickname: None,
            channels: HashMap::new(),
            backlog: Queue::new(MAX_BACKLOG),
        }
    }

    pub fn is_from_self(&self, msg: &Message) -> bool {
        if let Some(Prefix::User { nick, .. }) = &msg.prefix {
            if let Some(current) = &self.nickname {
                return current == nick;
            }
        }
        false
    }

    pub fn new_channel(&mut self, channel: &str) {
        // TODO this should never happen
        if self.channels.contains_key(channel) {
            warn!("joined existing channel: {}", channel);
            return;
        }

        let chan = Arc::new(Channel::new());
        self.channels.insert(channel.to_owned(), chan);
    }

    pub fn nick_join(&self, channel: &str, nick: &str) {
        // TODO this should never happen
        if !self.channels.contains_key(channel) {
            warn!("user: {} joined a missing channel: {}", nick, channel);
            return;
        }

        self.channels[channel].add_user(nick)
    }

    pub fn remove_channel(&mut self, channel: &str) {
        // TODO this should never happen
        if !self.channels.contains_key(channel) {
            warn!("left a missing channel: {}", channel);
            return;
        }

        self.channels.remove(channel);
    }

    pub fn nick_part(&self, channel: &str, nick: &str) {
        // TODO this should never happen
        if !self.channels.contains_key(channel) {
            warn!("user: {} left a missing channel: {}", nick, channel);
            return;
        }

        self.channels[channel].remove_user(&nick)
    }

    pub fn update_nick(&self, old: &str, nick: &str) {
        for channel in self.channels.values() {
            // this isn't needed. but it makes the logic apparent
            if channel.has_user(&old) {
                channel.update_user(&old, &nick)
            }
        }
    }

    pub fn remove_nick(&self, nick: &str) {
        for channel in self.channels.values() {
            channel.remove_user(&nick)
        }
    }
}

// TODO don't expose this
pub(crate) struct Inner {
    pub(crate) read: TcpStream,
    pub(crate) write: Mutex<TcpStream>,
}

impl Inner {
    fn get_nick_for(msg: &Message) -> &str {
        match &msg.prefix {
            Some(Prefix::User { nick, .. }) => nick,
            _ => unreachable!(),
        }
    }

    #[allow(unused_variables)]
    fn update(&mut self, msg: &Message, state: &Arc<RwLock<State>>) {
        let state = &mut state.write().unwrap();
        let from_self = state.is_from_self(&msg);

        match &msg.command {
            Command::Ping { token } => self.pong(token),

            // TODO target here is a vec..
            Command::Join { target, key } => {
                if from_self {
                    state.new_channel(&target[0]);
                    return;
                }

                let nick = Self::get_nick_for(&msg);
                state.nick_join(&target[0], &nick);
            }

            // TODO target here is a vec..
            Command::Part { target, reason } => {
                if from_self {
                    state.remove_channel(&target[0]);
                    return;
                }

                let nick = Self::get_nick_for(&msg);
                state.nick_part(&target[0], &nick);
            }

            Command::Quit { reason } => {
                if from_self {
                    // let the client clean up
                    return;
                }

                let nick = Self::get_nick_for(&msg);
                state.remove_nick(&nick)
            }

            Command::Nick { nickname } => {
                if from_self {
                    state.nickname = Some(nickname.to_owned());
                    return;
                }

                let old = Self::get_nick_for(&msg);
                state.update_nick(&old, &nickname);
            }

            Command::Other { command, params } => {
                // need to periodically do a /who or /names #channel
            }

            Command::Reply { numeric, params } => match numeric {
                1 => {
                    let name = &params[0];
                    state.nickname = Some(name.to_owned())
                }
                433 => self.nick(format!("{}_", params[1])),
                // TODO more numeric
                _ => {}
            },
            _ => {}
        };
    }
}

pub struct Client {
    pub(crate) state: Arc<RwLock<State>>,
    pub(crate) inner: Arc<RwLock<Inner>>,
}

impl Client {
    pub fn new(addr: impl AsRef<str>) -> Result<Self, Error> {
        let conn = TcpStream::connect(addr.as_ref()).map_err(Error::CannotConnect)?;

        let read = conn.try_clone().expect("conn clone for read");
        let write = conn.try_clone().expect("conn clone for write");

        Ok(Self {
            state: Arc::new(RwLock::new(State::new())),
            inner: Arc::new(RwLock::new(Inner {
                read,
                write: Mutex::new(write),
            })),
        })
    }

    pub fn run(&self) -> Receiver<Error> {
        let (err_tx, err_rx) = channel();

        let state = Arc::clone(&self.state);
        let inner = Arc::clone(&self.inner);

        thread::spawn(move || {
            let reader = {
                inner
                    .read()
                    .unwrap()
                    .read
                    .try_clone() // this is needed so the mutex isn't held
                    .expect("conn clone for reader")
            };

            for line in BufReader::new(reader).lines() {
                let line = match line.map_err(Error::CannotRead) {
                    Ok(line) => line,
                    Err(err) => {
                        debug!("error reading: {:?}", err);
                        err_tx.send(err).unwrap();
                        return;
                    }
                };
                trace!("<< {}", line.trim());

                let msg = match Message::parse(&line).map_err(Error::ParseError) {
                    Ok(msg) => msg,
                    Err(err) => {
                        debug!("error parsing: {:?}", err);
                        err_tx.send(err).unwrap();
                        return;
                    }
                };

                {
                    let inner = &mut inner.write().unwrap();
                    inner.update(&msg, &Arc::clone(&state));
                }

                let now = Instant::now();
                state.write().unwrap().backlog.push((now, msg));
            }

            err_tx.send(Error::EndOfStream).unwrap();
            trace!("end of read loop");
        });

        err_rx
    }

    pub fn join_channel(&self, chan: impl AsRef<str>) -> bool {
        {
            let state = &*self.state.read().unwrap();
            if state.channels.contains_key(chan.as_ref()) {
                return false;
            }
        }

        {
            let inner = &mut self.inner.write().unwrap();
            inner.join(chan, None);
        }
        true
    }

    pub fn leave_channel(&self, chan: impl AsRef<str>, reason: impl AsRef<str>) -> bool {
        {
            let state = &*self.state.read().unwrap();
            if !state.channels.contains_key(chan.as_ref()) {
                return false;
            }
        }

        {
            let inner = &mut self.inner.write().unwrap();
            inner.part(chan, reason);
        }

        true
    }

    pub fn is_from_self(&self, msg: &Message) -> bool {
        let state = &*self.state.read().unwrap();
        state.is_from_self(msg)
    }

    pub fn nickname(&self) -> Option<String> {
        let state = &*self.state.read().unwrap();
        // TODO get rid of this copy
        state.nickname.as_ref().cloned()
    }

    // XXX: need to incorporate these into the GUI state
    pub fn next_message(&self) -> Option<(Instant, Message)> {
        let state = &mut self.state.write().unwrap();
        state.backlog.pop()
    }

    pub fn has_queued_message(&self) -> bool {
        let state = &*self.state.write().unwrap();
        !state.backlog.is_empty()
    }

    pub fn get_channel(&self, name: impl AsRef<str>) -> Option<Arc<Channel>> {
        let state = &*self.state.read().unwrap();
        state.channels.get(name.as_ref()).map(Arc::clone)
    }
}
