use std::sync::Arc;

use super::*;
use riirc::IrcClient;

pub enum RunState {
    Continue,
    Exit,
}

pub struct Gui {
    container: pancurses::Window,
    output: OutputWindow,
    input: InputWindow,

    state: Arc<State>,
    queue: Arc<MessageQueue>,
    commands: Processor,
}

impl Gui {
    pub fn new() -> Self {
        let container = pancurses::initscr();
        pancurses::start_color();
        pancurses::use_default_colors();

        let bounds = container.get_max_yx();
        trace!("bounds: {:?}", bounds);

        let queue = Arc::new(MessageQueue::new());
        let state = Arc::new(State::new(Arc::clone(&queue)));

        let output = container
            .subwin(bounds.0, bounds.1, 0, 0)
            .expect("create output subwindow");
        output.setscrreg(0, output.get_max_y());
        output.scrollok(true);

        let input = container
            .subwin(1, bounds.1, bounds.0 - 1, 0)
            .expect("create input subwindow");

        pancurses::curs_set(1);
        pancurses::noecho();
        container.refresh();

        Self {
            container,
            output: OutputWindow::new(output),
            input: InputWindow::new(input, Arc::clone(&queue)),
            commands: Processor::new(Arc::clone(&state), Arc::clone(&queue)),
            state: Arc::clone(&state),
            queue,
        }
    }

    pub fn run(&mut self) -> RunState {
        use pancurses::Input::*;

        match self.input.read_input() {
            Ok(rt) => match rt {
                ReadType::Line(line) => {
                    if let Err(err) = self.commands.handle(&line) {
                        debug!("command error: {:?}", err);
                        return RunState::Exit;
                    }
                    self.input.clear_input();
                }
                ReadType::FKey(key) if key == KeyF10 => return RunState::Exit,
                ReadType::FKey(key) => trace!("fkey: {:?}", key),
                ReadType::None => if !self.read_buffers() {
                    // wipe out the state
                    debug!("resetting the state");
                    self.state.reset();

                    self.state.activate_buffer(0);
                    self.queue.status("Disconnected from server: quit");
                },
            },
            Err(err) => trace!("unknown: {:?}", err),
        };

        RunState::Continue
    }

    // false == restart
    fn read_buffers(&self) -> bool {
        if !self.read_errors() {
            return false;
        }

        self.read_queue();
        self.update_state();

        true
    }

    // false == restart
    fn read_errors(&self) -> bool {
        if let Some(errors) = self.state.read_errors() {
            if let Ok(err) = errors.try_recv() {
                debug!("error from irc client: {:?}", err);
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
                            self.output.writeln(&msg);
                        }
                    }
                }

                Request::Target(pos, data) => {
                    let (index, _) = self.state.current_buffer();
                    if index == pos {
                        self.output.writeln(&data);
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
                        let msg = format!("cannot part from {}. not on channel", &chan);
                        self.queue.status(&msg)
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

    fn update_state(&self) {
        self.state.sync_state()
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        pancurses::endwin();
    }
}
