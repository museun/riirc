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
    state: Arc<State>,
    inner: Arc<Inner>,
    errors: channel::Receiver<Error>,
}

impl Client {
    pub fn connect(addr: impl AsRef<str>) -> Result<Self, Error> {
        let (err_tx, err_rx) = channel::bounded(8);

        let this = Self {
            state: Arc::new(State::new()),
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
                state.push_message((Instant::now(), msg));
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
        if self.state.has_channel(chan.as_ref()) {
            return false;
        }
        self.inner.join(chan, None);
        true
    }

    pub fn leave_channel(&self, chan: impl AsRef<str>, reason: impl AsRef<str>) -> bool {
        if !self.state.has_channel(chan.as_ref()) {
            return false;
        }

        self.inner.part(chan, reason);
        true
    }

    pub fn state(&self) -> Arc<State> {
        Arc::clone(&self.state)
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
    fn update(&self, msg: &Message, state: &Arc<State>) {
        let from_self = state.is_from_self(&msg);

        match &msg.command {
            Command::Ping { token } => self.pong(token),

            Command::Join { channel, key: _key } => {
                if from_self {
                    state.new_channel(&channel);
                    return;
                }

                let nick = msg.get_nick();
                state.nick_join(&channel, &nick);
            }

            Command::Part {
                channel,
                reason: _reason,
            } => {
                if from_self {
                    state.remove_channel(&channel);
                    return;
                }

                let nick = msg.get_nick();
                state.nick_part(&channel, &nick);
            }

            Command::Quit { reason: _reason } => {
                if from_self {
                    // let the client clean up
                    return;
                }

                let nick = msg.get_nick();
                state.remove_nick(&nick)
            }

            Command::Nick { nickname } => {
                if from_self {
                    state.set_nickname(nickname);
                    return;
                }

                let old = msg.get_nick();
                state.update_nick(&old, &nickname);
            }

            Command::Other {
                command: _command,
                params: _params,
            } => {
                // need to periodically do a /who or /names #channel
            }

            Command::Reply { numeric, params } => match numeric {
                1 => {
                    let name = &params[0];
                    state.set_nickname(name);
                }
                433 => self.nick(format!("{}_", params[1])),
                // TODO more numerics
                _ => {}
            },

            _ => {
                // what should be done here?
            }
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
