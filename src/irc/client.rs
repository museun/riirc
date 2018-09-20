use super::message::Error as MessageError;
use super::*;

use crossbeam_channel as channel;

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Instant;

#[derive(Debug, PartialEq)]
pub enum Error {
    CannotConnect,
    ParseError(MessageError),
    CannotRead,
    EndOfStream,
}

pub struct Client {
    state: Arc<RwLock<State>>,
    inner: Arc<Inner>,
    errors: channel::Receiver<Error>,
}

impl Client {
    pub fn connect(addr: impl AsRef<str>) -> Result<Self, Error> {
        let (err_tx, err_rx) = channel::bounded(8);

        let this = Self {
            state: Arc::new(RwLock::new(State::new())),
            inner: Arc::new(Inner {
                stream: Mutex::new(None),
                buf: RwLock::new(VecDeque::new()),
            }),
            errors: err_rx,
        };

        let state = Arc::clone(&this.state);
        let inner = Arc::clone(&this.inner);
        let addr = addr.as_ref().to_string();

        thread::spawn(move || {
            // TODO do something with the inner error
            let conn = match TcpStream::connect(&addr).map_err(|_err| Error::CannotConnect) {
                Ok(conn) => conn,
                Err(err) => {
                    err_tx.send(err);
                    return;
                }
            };

            let read = conn.try_clone().expect("conn clone for read");
            let write = conn.try_clone().expect("conn clone for write");

            {
                *inner.stream.lock().unwrap() = Some(write);
                inner.flush();
            }

            for line in BufReader::new(read).lines() {
                let line = match line.map_err(|_err| Error::CannotRead) {
                    Ok(line) => line,
                    Err(err) => {
                        debug!("error reading: {:?}", err);
                        err_tx.send(err);
                        return;
                    }
                };
                trace!("<< {}", line.trim());

                let msg = match Message::parse(&line).map_err(Error::ParseError) {
                    Ok(msg) => msg,
                    Err(err) => {
                        debug!("error parsing: {:?}", err);
                        err_tx.send(err);
                        return;
                    }
                };

                inner.update(&msg, &Arc::clone(&state));

                let now = Instant::now();
                state.write().unwrap().backlog.push((now, msg));
            }

            err_tx.send(Error::EndOfStream);
            trace!("end of read loop");
        });

        Ok(this)
    }

    pub fn errors(&self) -> channel::Receiver<Error> {
        self.errors.clone()
    }

    pub fn join_channel(&self, chan: impl AsRef<str>) -> bool {
        {
            let state = &*self.state.read().unwrap();
            if state.channels.contains_key(chan.as_ref()) {
                return false;
            }
        }

        self.inner.join(chan, None);
        true
    }

    pub fn leave_channel(&self, chan: impl AsRef<str>, reason: impl AsRef<str>) -> bool {
        {
            let state = &*self.state.read().unwrap();
            if !state.channels.contains_key(chan.as_ref()) {
                return false;
            }
        }

        self.inner.part(chan, reason);
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

impl IrcClient for Client {
    // this should buffer the messages
    fn write(&self, data: &[u8]) {
        self.inner.write(data);
    }

    fn close(&self) {
        self.inner.close();
    }
}

struct Inner {
    stream: Mutex<Option<TcpStream>>,
    buf: RwLock<VecDeque<Vec<u8>>>,
}

impl Inner {
    fn get_nick_for(msg: &Message) -> &str {
        match &msg.prefix {
            Some(Prefix::User { nick, .. }) => nick,
            _ => unreachable!(),
        }
    }

    #[allow(unused_variables)]
    fn update(&self, msg: &Message, state: &Arc<RwLock<State>>) {
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

    fn flush(&self) {
        for msg in { self.buf.write().unwrap().drain(..) } {
            self.write(&msg);
        }
    }
}

impl IrcClient for Inner {
    fn write(&self, data: &[u8]) {
        use std::io::Write;
        use std::str;

        if self.stream.lock().unwrap().is_none() {
            trace!(
                "queueing: {}",
                str::from_utf8(&data[..data.len() - 2]).expect("valid utf-8")
            );
            self.buf.write().unwrap().push_back(data.to_vec());
            return;
        }

        let w = self.stream.lock().unwrap();
        let w = &mut w.as_ref().unwrap();
        trace!(
            ">> {}",
            str::from_utf8(&data[..data.len() - 2]).expect("valid utf-8")
        );
        // TODO split this as 510 chunks (512 - CRLF)
        w.write_all(data).expect("write")
    }

    fn close(&self) {
        use std::net::Shutdown;
        if let Some(writer) = &*self.stream.lock().unwrap() {
            writer.shutdown(Shutdown::Both).expect("shutdown TcpStream");
        }
    }
}
