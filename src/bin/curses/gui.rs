use std::sync::{Arc, RwLock};

use super::*;
use riirc::IrcClient;

type ShouldNotExit = bool;

// it supports utf-8 but it doesn't change the codepage in windows
// so we should probably do that before we init the library
pub struct Gui {
    container: Arc<pancurses::Window>,
    output: OutputWindow,
    input: InputWindow,
    nicklist: NicklistWindow,

    config: Arc<RwLock<Config>>,
    state: Arc<State>,
    queue: Arc<MessageQueue<Request>>,
    commands: Processor,
}

impl Gui {
    pub fn new(config: Config) -> Self {
        let container = pancurses::initscr();
        pancurses::start_color();
        pancurses::use_default_colors();

        let max = pancurses::COLOR_PAIRS();
        let (l, r, s, h) = if max >= 16 {
            (0x0F, 0xF0, 0x04, 0x10)
        } else {
            (0x07, 0x38, 0x03, 0x08)
        };

        for n in (0..max).map(|s| s as i16) {
            if n < h {
                pancurses::init_pair(n, n, -1);
            } else {
                pancurses::init_pair(n, n & l, (n & r) >> s);
            }
        }

        pancurses::curs_set(1);
        pancurses::noecho();

        let config = Arc::new(RwLock::new(config));
        let queue = Arc::new(MessageQueue::new());
        let state = Arc::new(State::new(Arc::clone(&queue), Arc::clone(&config)));

        let bounds = container.get_max_yx();
        trace!("bounds: {:?}", bounds);

        let container = Arc::new(container);
        let output = InnerWindow::new(
            OutputWindow::create,
            Arc::clone(&container),
            Arc::clone(&state),
        );

        let input = InnerWindow::new(
            InputWindow::create,
            Arc::clone(&container),
            Arc::clone(&state),
        );

        let nicklist = NicklistWindow::new(Arc::clone(&container));

        container.refresh();

        Self {
            container,
            output,
            input,
            nicklist,

            commands: Processor::new(Arc::clone(&state), Arc::clone(&queue)),
            config: Arc::clone(&config),
            state: Arc::clone(&state),
            queue,
        }
    }

    pub fn run(&mut self) {
        use pancurses::Input::*;

        loop {
            match self.input.read_input() {
                ReadType::Line(line) => {
                    use super::command::Error::*;
                    if let Err(err) = self.commands.dispatch(&line) {
                        debug!("command error: {:?}", err);
                        match err {
                            // TODO output-ize this
                            InvalidArgument(s) | InvalidBuffer(s) => {
                                self.output(Output::new().add(s).build(), true)
                            }
                            ClientError(err) => {
                                let output = Output::new()
                                    .fg(Color::Red)
                                    .add("error: ")
                                    .add("irc client error: ")
                                    .fg(Color::Cyan)
                                    .add(format!("{:?}", err))
                                    .build();
                                self.output(output, true);
                            }
                            AlreadyConnected => {
                                let output = Output::new()
                                    .fg(Color::Red)
                                    .add("error: ")
                                    .add("already connected")
                                    .build();
                                self.output(output, true)
                            }
                            NotConnected => {
                                let output = Output::new()
                                    .fg(Color::Red)
                                    .add("error: ")
                                    .add("not connected")
                                    .build();
                                self.output(output, true)
                            }
                            ForceExit => break,
                        }
                    };
                    self.input.add_history();
                    self.input.clear_input();
                }

                // TODO merge this stuff
                ReadType::FKey(key) if key == KeyF10 => break,
                ReadType::FKey(key) => trace!("fkey: {:?}", key),

                ReadType::None => if !self.read_buffers() {
                    debug!("resetting the state");
                    // flush the queue before clearing
                    self.read_queue();
                    // wipe out the state
                    self.state.reset();
                    self.state.activate_buffer(0);
                },
            }
        }

        self.config.read().unwrap().save();
    }

    fn read_buffers(&mut self) -> ShouldNotExit {
        if !self.read_errors() {
            return false;
        }

        self.read_queue();
        self.state.update();
        true
    }

    fn read_errors(&self) -> ShouldNotExit {
        if let Some(errors) = self.state.read_errors() {
            if let Some(err) = errors.try_recv() {
                let output = Output::new()
                    .fg(Color::Red)
                    .add("error: ")
                    .add("irc c;ient error ")
                    .fg(Color::Cyan)
                    .add(format!("{:?}", err))
                    .build();

                self.output(output, true);
                return false;
            }
        };
        true
    }

    fn read_queue(&mut self) {
        if self.queue.len() == 0 {
            return;
        }

        for req in self.queue.read_all() {
            match req {
                Request::Queue(pos, data) => {
                    if let Some(buf) = self.state.get_buffer(pos) {
                        buf.push_message(&data);
                    }

                    let (index, buf) = self.state.current_buffer();
                    if index == pos {
                        if let Some(msg) = buf.most_recent() {
                            self.output.window().output(&msg, true);
                        }
                    }
                }

                Request::Target(pos, data) => {
                    let (index, _) = self.state.current_buffer();
                    if index == pos {
                        self.output.output(&data, true);
                    }
                }

                Request::Clear(scrollback) => {
                    self.output.clear();
                    if scrollback {
                        let (_, buf) = self.state.current_buffer();
                        buf.clear();
                    }
                }

                Request::ToggleNickList => {
                    let (index, buf) = self.state.current_buffer();
                    if index == 0 {
                        return;
                    }

                    if self.nicklist.is_visible() {
                        self.nicklist.toggle();
                        return;
                    }

                    // TODO get rid of these allocations
                    if let Some(client) = self.state.client() {
                        if let Some(ch) = client.state().get_channel(&buf.name()) {
                            self.nicklist.toggle();
                            self.nicklist
                                .update(&ch.users().iter().map(|s| s.as_str()).collect::<Vec<_>>());
                        }
                    }
                }

                // TODO: buffers don't have their own input windows
                Request::ClearHistory(_buf) => self.input.clear_history(),

                Request::SwitchBuffer(buf) => {
                    if buf == 0 && self.nicklist.is_visible() {
                        self.nicklist.toggle();
                        return;
                    }

                    self.state.activate_buffer(buf);
                }

                Request::NextBuffer => {
                    let len = self.state.buffers_len();
                    let (index, _) = self.state.current_buffer();
                    let pos = if index == len - 1 { 0 } else { index + 1 };
                    self.state.activate_buffer(pos);
                }

                Request::PrevBuffer => {
                    let len = self.state.buffers_len();
                    let (index, _) = self.state.current_buffer();
                    let pos = if index == 0 { len - 1 } else { index - 1 };
                    self.state.activate_buffer(pos);
                }

                Request::Join(chan) => {
                    let client = self.state.client().unwrap();
                    if !client.join_channel(&chan) {
                        self.state.activate_buffer_by_name(&chan)
                    } else {
                        self.state.new_buffer(&chan, true);
                    }
                }

                Request::Part(chan) => {
                    let client = self.state.client().unwrap();
                    if !client.leave_channel(&chan, "leaving") {
                        let output = Output::new()
                            .fg(Color::Red)
                            .add("error: ")
                            .add("cannot part from ")
                            .fg(Color::Cyan)
                            .add(chan)
                            .add(". not on channel")
                            .build();
                        self.output(output, true)
                    } else {
                        self.state.remove_buffer(&chan);
                    }
                }

                Request::Quit(msg) => {
                    let client = self.state.client().unwrap();
                    client.quit(msg);
                }
            };
        }
    }
}

impl Outputter for Gui {
    fn output(&self, output: impl Into<Output>, eol: bool) {
        self.queue.enqueue(Request::Queue(0, output.into()));
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        pancurses::endwin();
    }
}
