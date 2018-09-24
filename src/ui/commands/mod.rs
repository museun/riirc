use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{colors::Color, keybinds::*, output::Output, request::*, state::State, *};

import!(
    bind,
    buffer,
    clear,
    clear_history,
    connect,
    echo,
    exit,
    join,
    list_buffers,
    part,
    quit,
    rehash
);

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidArgument(String),
    InvalidBuffer(String),

    AlreadyConnected,
    NotConnected,
    ClientError(irc::Error),

    ReloadConfig,
    EmptyInput,
    UnknownCommand(String),

    ForceExit, // why is this an error?
}

pub enum Response {
    Nothing,
    Output(Output),
}

type CommandResult = Result<Response, Error>;
type Command = fn(&Context) -> CommandResult;

pub(crate) struct Context<'a> {
    pub(crate) state: Rc<State>,
    pub(crate) queue: Rc<MessageQueue<Request>>,
    pub(crate) config: Rc<RefCell<Config>>,
    pub(crate) parts: &'a [&'a str],
}

impl<'a> Context<'a> {
    pub fn request(&self, req: Request) {
        self.queue.enqueue(req)
    }

    pub fn status(&self, output: Output) {
        self.request(Request::Queue(0, output));
    }
}

pub struct CommandProcessor {
    map: HashMap<&'static str, Command>,
    state: Rc<State>,
    queue: Rc<MessageQueue<Request>>,
}

impl CommandProcessor {
    pub fn new(state: Rc<State>, queue: Rc<MessageQueue<Request>>) -> Self {
        let mut this = CommandProcessor {
            map: HashMap::new(),
            state,
            queue,
        };

        this.map.insert("/echo", echo_command);
        this.map.insert("/exit", exit_command);
        this.map.insert("/connect", connect_command);
        this.map.insert("/quit", quit_command);
        this.map.insert("/clear", clear_command);
        this.map.insert("/join", join_command);
        this.map.insert("/part", part_command);
        this.map.insert("/buffer", buffer_command);
        this.map.insert("/buffers", list_buffers_command);
        this.map.insert("/bind", bind_command);
        this.map.insert("/rehash", rehash_command);
        this.map.insert("/clearhistory", clear_history_command);
        this
    }

    pub fn dispatch(&mut self, input: &str) -> CommandResult {
        if input.is_empty() {
            return Err(Error::EmptyInput);
        }

        if !input.starts_with('/') {
            return self.try_send_message(input);
        }

        let input = input.to_string();
        let mut input = input.split(' ');
        let query = input.next().unwrap();
        if !self.map.contains_key(query) {
            return Err(Error::UnknownCommand(query.into()));
        }

        trace!("query: {}", query);

        let parts = input.collect::<Vec<_>>();
        let func = self.map[&query];
        let ctx = Context {
            state: Rc::clone(&self.state),
            queue: Rc::clone(&self.queue),
            config: Rc::clone(&self.state.config()),
            parts: &parts,
        };

        func(&ctx)
    }

    fn try_send_message(&self, data: &str) -> CommandResult {
        use super::irc::IrcClient;
        let client = self.state.client().ok_or_else(|| Error::NotConnected)?;

        let (_index, buffer) = self.state.buffers().current();
        if buffer.is_status() {
            return Err(Error::InvalidBuffer(buffer.name().into()));
        }

        client.privmsg(&buffer.name(), data);

        let nickname = client
            .state()
            .nickname()
            .expect("client should have a valid nickname");

        let mut output = Output::stamp();
        output.fg(Color::Green).add(nickname).add(" ").add(data);
        Ok(Response::Output(output.build()))
    }
}

fn assume_connected(ctx: &Context) -> Result<(), Error> {
    if ctx.state.client().is_none() {
        Err(Error::NotConnected)?;
    }
    Ok(())
}

fn assume_args(ctx: &Context, msg: &'static str) -> Result<(), Error> {
    if ctx.parts.is_empty() {
        Err(Error::InvalidArgument(msg.into()))?;
    }
    Ok(())
}
