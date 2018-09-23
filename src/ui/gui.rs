use std::cell::RefCell;
use std::rc::Rc;
use std::sync::RwLock;

use super::windows::*;
use super::*;

pub struct Gui {
    queue: Rc<MessageQueue<super::Request>>,
    container: Rc<RefCell<Container>>,
    events: EventProcessor,
    commands: CommandProcessor,
    state: Rc<State>,
}

impl Gui {
    pub fn new(config: IrcConfig) -> Self {
        let queue = Rc::new(MessageQueue::new());
        let state = Rc::new(State::new(
            Rc::clone(&queue),
            Rc::new(Config {
                0: RwLock::new(config),
            }),
        ));

        let container = Rc::new(RefCell::new(Container::new(Rc::clone(&state))));
        let events =
            EventProcessor::new(Rc::clone(&state), Rc::clone(&queue), Rc::clone(&container));
        let commands = CommandProcessor::new(Rc::clone(&state), Rc::clone(&queue));

        Self {
            queue,
            container,
            events,
            commands,
            state,
        }
    }

    pub fn run(&mut self) {
        loop {
            match {
                let s = self.container.borrow_mut().step();
                s.clone()
            } {
                windows::ReadType::Line(line) => {
                    if let Err(err) = self.commands.dispatch(&line) {
                        debug!("command error: {:?}", err);
                        match err {
                            // TODO output-ize this
                            ui::Error::InvalidArgument(s) | ui::Error::InvalidBuffer(s) => {
                                trace!("{:?}", ui::Output::new().add(s).build())
                            }
                            ui::Error::ClientError(err) => {
                                let output = ui::Output::new()
                                    .fg(ui::Color::Red)
                                    .add("error: ")
                                    .add("irc client error: ")
                                    .fg(ui::Color::Cyan)
                                    .add(format!("{:?}", err))
                                    .build();
                                trace!("{:?}", output);
                            }
                            ui::Error::AlreadyConnected => {
                                let output = ui::Output::new()
                                    .fg(ui::Color::Red)
                                    .add("error: ")
                                    .add("already connected")
                                    .build();
                                trace!("{:?}", output);
                            }
                            ui::Error::NotConnected => {
                                let output = ui::Output::new()
                                    .fg(ui::Color::Red)
                                    .add("error: ")
                                    .add("not connected")
                                    .build();
                                trace!("{:?}", output);
                            }
                            ui::Error::ForceExit => break,

                            _ => error!("unknown error: {:?}", err),
                        }
                    };

                    {
                        self.container.borrow_mut().add_and_clear();
                    }
                }

                // TODO merge this stuff
                windows::ReadType::FKey(key) if key == pancurses::Input::KeyF10 => break,
                windows::ReadType::FKey(key) => trace!("fkey: {:?}", key),

                windows::ReadType::None => if !self.read_buffers() {
                    debug!("resetting the state");
                    // flush the queue before clearing
                    self.events.process();

                    // wipe out the state
                    self.state.reset();
                    self.state.buffers().activate(0);
                },
            }
        }
    }

    fn read_buffers(&mut self) -> bool {
        if !self.read_errors() {
            return false;
        }

        self.events.process();
        true
    }

    fn read_errors(&mut self) -> bool {
        if let Some(errors) = self.state.read_errors() {
            if let Some(err) = errors.try_recv() {
                let output = ui::Output::new()
                    .fg(ui::Color::Red)
                    .add("error: ")
                    .add("irc c;ient error ")
                    .fg(ui::Color::Cyan)
                    .add(format!("{:?}", err))
                    .build();

                trace!("{:?}", output);
                return false;
            }
        };
        true
    }
}

// TODO remove this garbage
pub struct Config(RwLock<IrcConfig>);

impl Config {
    pub fn server(&self) -> String {
        self.0.read().unwrap().server.clone()
    }
    pub fn nick(&self) -> String {
        self.0.read().unwrap().nick.clone()
    }
    pub fn user(&self) -> String {
        self.0.read().unwrap().user.clone()
    }
    pub fn real(&self) -> String {
        self.0.read().unwrap().real.clone()
    }
    pub fn pass(&self) -> String {
        self.0.read().unwrap().pass.clone()
    }

    // this is very expensive..
    pub fn keybinds(&self) -> Keybinds {
        self.0.read().unwrap().keybinds.clone()
    }

    pub fn update(&self, kt: ui::KeyType, kr: ui::KeyRequest) {
        let r = &mut *self.0.write().unwrap();
        r.keybinds.insert(kt, kr)
    }

    pub fn load() -> Option<Self> {
        match IrcConfig::load("riirc.toml") {
            Ok(config) => Some(Self {
                0: RwLock::new(config),
            }),
            Err(err) => {
                error!("cannot load config: {}", err);
                None
            }
        }
    }

    pub fn save(&self) {
        self.0.read().unwrap().save()
    }

    pub fn replace(&self, config: Config) {
        let src = config.0.into_inner().expect("consume mutex");
        ::std::mem::replace(&mut *self.0.write().unwrap(), src);
    }
}
