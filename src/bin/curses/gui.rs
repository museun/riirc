use std::sync::{Arc, RwLock};

use super::*;
use riirc::IrcClient;

pub trait Window {
    fn window(&self) -> &pancurses::Window;

    fn output(&self, output: &Output, eol: bool) {
        for (r, cp) in output.colors.iter() {
            self.insert(*cp, &output.data[r.start..r.end], false)
        }
        if eol {
            self.window().addch('\n');
        }
    }

    // TODO impl background colors
    fn insert(&self, color: impl Into<ColorPair>, s: impl AsRef<str>, eol: bool) {
        let window = self.window();
        let (y, x) = window.get_cur_yx();

        let color = color.into();
        let s = s.as_ref();
        window.addstr(s);

        let (ny, nx) = window.get_cur_yx();
        window.mvchgat(
            y,
            x,
            s.len() as i32,
            if color.bold {
                pancurses::A_BOLD
            } else {
                pancurses::A_NORMAL
            },
            color.fg.into(),
        );
        window.mv(ny, nx);
        if eol {
            window.addch('\n');
        }
        window.refresh();
    }
}

type ShouldNotExit = bool;

// it supports utf-8 but it doesn't change the codepage in windows
// so we should probably do that before we init the library
pub struct Gui {
    container: pancurses::Window,
    output: OutputWindow,
    input: InputWindow,

    config: Arc<RwLock<Config>>,
    state: Arc<State>,
    queue: Arc<MessageQueue>,
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

        let output = container
            .subwin(bounds.0 - 1, bounds.1, 0, 0)
            .expect("create output subwindow");
        output.setscrreg(0, output.get_max_y());
        output.scrollok(true);
        output.color_set(0);

        let input = container
            .subwin(1, bounds.1, bounds.0 - 1, 0)
            .expect("create input subwindow");

        //    input.bkgd(pancurses::COLOR_PAIR(16 * 5));

        container.refresh();

        Self {
            container,
            output: OutputWindow::new(output),
            input: InputWindow::new(input, Arc::clone(&queue), Arc::clone(&config)),
            commands: Processor::new(Arc::clone(&state), Arc::clone(&queue)),

            config: Arc::clone(&config),
            state: Arc::clone(&state),
            queue,
        }
    }

    // fn dump_colors(&self, buf: usize) {
    //     for i in 0..pancurses::COLOR_PAIRS() {
    //         if i > 0 && i % 16 == 0 {
    //             self.output.addch('\n');
    //         }
    //         self.output.attrset(pancurses::COLOR_PAIR(i as u64));
    //         self.output.addch('@');
    //         self.output
    //             .attrset(pancurses::COLOR_PAIR(i as u64) | pancurses::A_BOLD);
    //         self.output.addch('#');
    //     }
    // }

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
                                self.queue.status(Output::new().add(s).build())
                            }
                            ClientError(err) => {
                                let output = Output::new()
                                    .fg(Color::Red)
                                    .add("error: ")
                                    .add("irc client error: ")
                                    .fg(Color::Cyan)
                                    .add(format!("{:?}", err))
                                    .build();
                                self.queue.status(output);
                            }
                            AlreadyConnected => {
                                let output = Output::new()
                                    .fg(Color::Red)
                                    .add("error: ")
                                    .add("already connected")
                                    .build();
                                self.queue.status(output)
                            }
                            NotConnected => {
                                let output = Output::new()
                                    .fg(Color::Red)
                                    .add("error: ")
                                    .add("not connected")
                                    .build();
                                self.queue.status(output)
                            }
                            ForceExit => break,
                        }
                    };
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

    fn read_buffers(&self) -> ShouldNotExit {
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

                self.queue.status(output);
                return false;
            }
        };
        true
    }

    fn read_queue(&self) {
        if self.queue.len() == 0 {
            return;
        }

        for req in self.queue.read_queue() {
            trace!("req: {:?}", req);
            match req {
                Request::Queue(pos, data) => {
                    if let Some(buf) = self.state.get_buffer(pos) {
                        buf.push_message(&data);
                    }

                    let (index, buf) = self.state.current_buffer();
                    if index == pos {
                        if let Some(msg) = buf.most_recent() {
                            self.output.output(&msg, true);
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

                Request::SwitchBuffer(buf) => {
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
                        self.queue.status(output)
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

impl Drop for Gui {
    fn drop(&mut self) {
        pancurses::endwin();
    }
}
