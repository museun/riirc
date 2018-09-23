use super::colors::Color;
use super::output::Output;
use super::request::Request;
use super::state::State;
use super::windows::Container;
use super::*;

use std::cell::RefCell;
use std::rc::Rc;

pub struct EventProcessor {
    state: Rc<State>,
    queue: Rc<MessageQueue<Request>>,
    container: Rc<RefCell<Container>>,
}

impl EventProcessor {
    pub fn new(
        state: Rc<State>,
        queue: Rc<MessageQueue<Request>>,
        container: Rc<RefCell<Container>>,
    ) -> Self {
        Self {
            state,
            queue,
            container,
        }
    }

    pub fn process(&self) {
        self.read_irc_events();
        self.read_requests();
    }

    // TODO finish this
    #[allow(unused_variables)]
    fn read_irc_events(&self) {
        let client = match self.state.client() {
            Some(client) => client,
            None => return,
        };

        let (ts, msg, me) = match {
            let state = client.state();
            (state.next_message(), state.nickname())
        } {
            (Some((ts, msg)), me) => (ts, msg, me),
            (None, _) => return,
        };

        let buffers = self.state.buffers();
        let (index, active) = buffers.current();

        macro_rules! send_to_buf {
            ($e:expr, $output:expr) => {
                if let Some(pos) = buffers.index_of($e) {
                    self.queue.enqueue(Request::Queue(pos, $output))
                } else {
                    warn!("unknown buffer: {} ({:?})", $e, &msg)
                }
            };
        }

        match &msg.command {
            IrcCommand::Privmsg {
                target,
                data,
                is_notice,
            }
                if !is_notice =>
            {
                send_to_buf!(
                    target,
                    Output::stamp()
                        .add(msg.get_nick())
                        .add(" ")
                        .add(data)
                        .build()
                );
            }
            IrcCommand::Privmsg { target, data, .. } => {
                send_to_buf!(
                    target,
                    Output::stamp()
                        .add(msg.get_nick())
                        .add(" ")
                        .add(data)
                        .build()
                );
            }

            IrcCommand::Join { channel, .. } => {
                send_to_buf!(
                    channel,
                    Output::stamp()
                        .add(msg.get_nick())
                        .add(" join ")
                        .add(channel)
                        .build()
                );
            }

            IrcCommand::Part { channel, reason } => {
                let mut output = Output::stamp();
                output.add(msg.get_nick()).add(" left ").add(channel);
                if reason.is_some() {
                    output.add(": ").add(reason.as_ref().unwrap());
                }

                send_to_buf!(channel, output.build());
            }

            _ => debug!("unknown: {:?}", msg),
        };
    }

    fn read_requests(&self) {
        let requests = self.state.read_requests();
        if requests.is_empty() {
            return;
        }

        for req in &requests {
            self.handle_request(req);
        }
    }

    fn handle_request(&self, req: &Request) -> Option<()> {
        use super::irc::IrcClient;

        match req {
            Request::Queue(pos, data) => {
                let buffers = self.state.buffers();
                if let Some(buf) = buffers.get(*pos) {
                    buf.push_message(&data);
                }

                let (index, buf) = buffers.current();
                if index == *pos {
                    let msg = buf.most_recent()?;
                    self.container.borrow().output().output(msg, true);
                }
            }

            Request::Target(pos, data) => {
                let buffers = self.state.buffers();
                let (index, _) = buffers.current();
                if index == *pos {
                    self.container.borrow().output().output(data.clone(), true);
                }
            }

            Request::Clear(scrollback) => {
                let buffers = self.state.buffers();
                self.container
                    .borrow()
                    .output()
                    .queue(windows::Request::Clear(None));
                if *scrollback {
                    let (_, buf) = buffers.current();
                    buf.clear();
                }
            }

            Request::ToggleNickList => {
                let buffers = self.state.buffers();
                let (index, buf) = buffers.current();
                if index == 0 {
                    return None;
                }

                let nicklist = self.container.borrow().nicklist();
                if nicklist.is_visible() {
                    nicklist.toggle();
                    return None;
                }

                let ch = self.state.client()?.state().channels().get(&buf.name())?;
                nicklist.toggle();
                for user in ch.users() {
                    nicklist.output(Output::new().add(&user.as_ref()).build(), true)
                }
            }

            Request::ClearHistory(_buf) => {
                //self.container.input().clear_history()
            }

            Request::SwitchBuffer(buf) => {
                let nicklist = self.container.borrow().nicklist();
                if *buf == 0 && nicklist.is_visible() {
                    nicklist.toggle();
                    return None;
                }

                let buffers = self.state.buffers();
                buffers.activate(*buf)
            }

            Request::NextBuffer => {
                let buffers = self.state.buffers();
                let len = buffers.len();
                let (index, _) = buffers.current();
                let pos = if index == len - 1 { 0 } else { index + 1 };
                buffers.activate(pos);
            }

            Request::PrevBuffer => {
                let buffers = self.state.buffers();
                let len = buffers.len();
                let (index, _) = buffers.current();
                let pos = if index == 0 { len - 1 } else { index - 1 };
                buffers.activate(pos);
            }

            Request::Join(ref chan) => {
                let buffers = self.state.buffers();
                if buffers.named(&chan).is_some() {
                    buffers.activate_by_name(&chan)
                } else {
                    self.state.client()?.part(&chan, "leaving");
                    buffers.create(&chan, true);
                }
            }

            Request::Part(ref chan) => {
                let buffers = self.state.buffers();
                if buffers.named(&chan).is_some() {
                    self.state.client()?.part(&chan, "leaving");
                    buffers.delete(&chan);
                } else {
                    self.container.borrow().output().output(
                        Output::new()
                            .add("not on ")
                            .fg(Color::Cyan)
                            .add(&chan)
                            .build(),
                        false,
                    );
                }
            }

            Request::Quit(msg) => self.state.client()?.quit(msg.clone()),
        };

        None
    }
}
