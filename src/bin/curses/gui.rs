use super::ui::Outputter;
use super::windowlocator::WindowLocator;
use super::*;

use std::sync::{Arc, RwLock};

type ShouldNotExit = bool;

// it supports utf-8 but it doesn't change the codepage in windows
// so we should probably do that before we init the library
pub struct Gui {
    container: CWindow,
    locator: Arc<WindowLocator>,
    events: ui::EventProcessor,
    config: Arc<RwLock<riirc::Config>>,
    state: Arc<ui::State>,
    queue: Arc<ui::MessageQueue<ui::Request>>,
    commands: ui::Processor,
}

impl Gui {
    pub fn new(config: riirc::Config) -> Self {
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
        let queue = Arc::new(ui::MessageQueue::new());
        let state = Arc::new(ui::State::new(Arc::clone(&queue), Arc::clone(&config)));

        let bounds = container.get_max_yx();
        trace!("bounds: {:?}", bounds);

        let container: CWindow = container.into();
        let output = OutputWindow::new(container.clone(), Arc::clone(&state));
        let input = InputWindow::new(container.clone(), Arc::clone(&state));
        let nicklist = NicklistWindow::new(container.clone(), Arc::clone(&state));
        container.refresh();

        let locator = Arc::new(WindowLocator::new(
            Arc::new(output),
            Arc::new(input),
            Arc::new(nicklist),
        ));

        Self {
            container,
            locator: Arc::clone(&locator),
            events: ui::EventProcessor::new(Arc::clone(&state), Arc::clone(&queue), locator),
            commands: ui::Processor::new(Arc::clone(&state), Arc::clone(&queue)),
            config: Arc::clone(&config),
            state,
            queue,
        }
    }

    pub fn run(&mut self) {
        use super::ui::commands::Error::*;
        use super::ui::WindowLocator;

        loop {
            let input = self.locator.input();

            match input.read_input() {
                ui::ReadType::Line(line) => {
                    if let Err(err) = self.commands.dispatch(&line) {
                        debug!("command error: {:?}", err);
                        match err {
                            // TODO output-ize this
                            InvalidArgument(s) | InvalidBuffer(s) => {
                                self.output(ui::Output::new().add(s).build(), true)
                            }
                            ClientError(err) => {
                                let output = ui::Output::new()
                                    .fg(ui::Color::Red)
                                    .add("error: ")
                                    .add("irc client error: ")
                                    .fg(ui::Color::Cyan)
                                    .add(format!("{:?}", err))
                                    .build();
                                self.output(output, true);
                            }
                            AlreadyConnected => {
                                let output = ui::Output::new()
                                    .fg(ui::Color::Red)
                                    .add("error: ")
                                    .add("already connected")
                                    .build();
                                self.output(output, true)
                            }
                            NotConnected => {
                                let output = ui::Output::new()
                                    .fg(ui::Color::Red)
                                    .add("error: ")
                                    .add("not connected")
                                    .build();
                                self.output(output, true)
                            }
                            ForceExit => break,

                            _ => error!("unknown error: {:?}", err),
                        }
                    };

                    input.add_history();
                    input.clear_input();
                }

                // TODO merge this stuff
                ui::ReadType::FKey(key) if key == ui::FKey::F10 => break,
                ui::ReadType::FKey(key) => trace!("fkey: {:?}", key),

                ui::ReadType::None => if !self.read_buffers() {
                    debug!("resetting the state");
                    // flush the queue before clearing
                    self.events.process();

                    // wipe out the state
                    self.state.reset();
                    self.state.buffers().activate(0);
                },
            }
        }

        self.config.read().unwrap().save();
    }

    fn read_buffers(&mut self) -> ShouldNotExit {
        if !self.read_errors() {
            return false;
        }

        self.events.process();
        true
    }

    fn read_errors(&self) -> ShouldNotExit {
        if let Some(errors) = self.state.read_errors() {
            if let Some(err) = errors.try_recv() {
                let output = ui::Output::new()
                    .fg(ui::Color::Red)
                    .add("error: ")
                    .add("irc c;ient error ")
                    .fg(ui::Color::Cyan)
                    .add(format!("{:?}", err))
                    .build();

                self.output(output, true);
                return false;
            }
        };
        true
    }
}

impl ui::Outputter for Gui {
    fn output(&self, output: ui::Output, _eol: bool) {
        self.queue.enqueue(ui::Request::Queue(0, output.into()));
    }

    fn clear(&self) {
        unimplemented!()
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        pancurses::endwin();
    }
}
