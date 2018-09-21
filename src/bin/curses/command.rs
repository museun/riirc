use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::{Arc, RwLock};

use super::*;
use riirc::IrcClient;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidArgument(&'static str),
    InvalidBuffer(&'static str),
    AlreadyConnected,
    NotConnected,
    ClientError(riirc::IrcError),
    ForceExit,
}

// TODO make a help system
// TODO TODO deserialize it from a file

type CommandResult = Result<(), Error>;
type Command = fn(&Context) -> CommandResult;

struct Context<'a> {
    state: Arc<State>,
    queue: Arc<MessageQueue>,
    config: Arc<RwLock<Config>>,
    parts: &'a [&'a str],
}

pub struct Processor {
    map: HashMap<&'static str, Command>,
    state: Arc<State>,
    queue: Arc<MessageQueue>,
}

impl Processor {
    pub fn new(state: Arc<State>, queue: Arc<MessageQueue>) -> Self {
        let mut this = Self {
            map: HashMap::new(),
            state,
            queue,
        };

        this.initialize();
        this
    }

    fn initialize(&mut self) {
        self.map.insert("/echo", echo_command);
        self.map.insert("/exit", |ctx| {
            if let Some(client) = ctx.state.client() {
                client.quit(Some("leaving".into()));
            }
            Err(Error::ForceExit)
        });
        self.map.insert("/connect", connect_command);
        self.map.insert("/quit", quit_command);
        self.map.insert("/clear", clear_command);
        self.map.insert("/join", join_command);
        self.map.insert("/part", part_command);
        self.map.insert("/buffer", buffer_command);
        self.map.insert("/buffers", list_buffers_command);
        self.map.insert("/bind", bind_command);
        self.map.insert("/rehash", rehash_command);
        self.map.insert("/clearhistory", clear_history_command);
    }

    pub fn dispatch(&mut self, input: &str) -> CommandResult {
        if input.is_empty() {
            return Ok(());
        }

        if !input.starts_with('/') {
            self.state.send_line(input);
            return Ok(());
        }

        let input = input.to_string();
        let mut input = input.split(' ');
        let query = input.next().unwrap();
        if !self.map.contains_key(query) {
            let output = Output::new()
                .fg(Color::Red)
                .add("error: ")
                .add("unknown command: ")
                .fg(Color::Cyan)
                .add(query)
                .build();
            self.queue.status(output);
            return Ok(());
        }

        trace!("query: {}", query);

        let parts = input.collect::<Vec<_>>();
        let func = self.map[&query];
        let ctx = Context {
            state: Arc::clone(&self.state),
            queue: Arc::clone(&self.queue),
            config: Arc::clone(&self.state.get_config()),
            parts: &parts,
        };

        func(&ctx)
    }
}

// this isn't a command
fn assume_connected(ctx: &Context) -> CommandResult {
    if ctx.state.client().is_none() {
        Err(Error::NotConnected)?;
    }
    Ok(())
}

fn assume_args(ctx: &Context, msg: &'static str) -> CommandResult {
    if ctx.parts.is_empty() {
        Err(Error::InvalidArgument(msg))?;
    }
    Ok(())
}

fn echo_command(ctx: &Context) -> CommandResult {
    for part in ctx.parts {
        let output = Output::new().add(*part).build();
        ctx.queue.status(output);
    }
    Ok(())
}

fn connect_command(ctx: &Context) -> CommandResult {
    if ctx.state.client().is_some() {
        Err(Error::AlreadyConnected)?;
    };

    let config = ctx.config.read().unwrap();

    let output = Output::new()
        .fg(Color::Green)
        .add("connecting to ")
        .fg(Color::Cyan)
        .add(&config.server)
        .build();
    ctx.queue.status(output);

    let client = riirc::Client::connect(&config.server).map_err(Error::ClientError)?;
    if !&config.pass.is_empty() {
        client.pass(&config.pass)
    }
    client.nick(&config.nick);
    client.user(&config.user, &config.real);
    ctx.state.set_client(client);
    Ok(())
}

fn quit_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;

    let msg = if ctx.parts.is_empty() {
        None
    } else {
        Some(ctx.parts.join(" "))
    };

    ctx.queue.request(Request::Quit(msg));
    Ok(())
}

fn join_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;
    assume_args(&ctx, "try: /join <chan>")?;

    // TODO make this actually work on multiple channerls + keys
    ctx.queue.request(Request::Join(ctx.parts[0].to_owned()));
    Ok(())
}

fn part_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;

    let (status, buf) = ctx.state.at_status_buffer();
    if status {
        Err(Error::InvalidBuffer("cannot /part in a *window"))?;
    };

    let ch = if ctx.parts.is_empty() {
        buf.name().to_string()
    } else {
        ctx.parts[0].to_string()
    };

    ctx.queue.request(Request::Part(ch));
    Ok(())
}

fn clear_command(ctx: &Context) -> CommandResult {
    ctx.queue.request(Request::Clear(true));
    Ok(())
}

fn buffer_command(ctx: &Context) -> CommandResult {
    assume_args(&ctx, "try: /buffer N")?;

    let buf = ctx.parts[0]
        .parse::<usize>()
        .map_err(|_e| Error::InvalidArgument("try: /buffer N (a number this time)"))?;

    ctx.queue.request(Request::SwitchBuffer(buf));
    Ok(())
}

fn list_buffers_command(ctx: &Context) -> CommandResult {
    let buffers = ctx.state.buffers();
    let len = ctx.state.buffers_len() - 1;

    let mut output = Output::new();
    output.fg(Color::White).add("buffers: ");

    for (n, buffer) in buffers.iter().enumerate() {
        output
            .fg(Color::BrightWhite)
            .add(format!("{}", n))
            .fg(Color::Cyan)
            .add(buffer.name());

        if n < len {
            output.add(",");
        }
    }

    ctx.queue.status(output.build());
    Ok(())
}

fn bind_command(ctx: &Context) -> CommandResult {
    match (ctx.parts.get(0), ctx.parts.get(1)) {
        (None, None) => {
            let keybinds = &ctx.config.read().unwrap().keybinds;
            for (k, v) in keybinds.iter() {
                let output = Output::new()
                    .fg(Color::Yellow)
                    .add(k.to_string())
                    .add(" -> ")
                    .fg(Color::Cyan)
                    .add(v.to_string())
                    .build();
                ctx.queue.status(output)
            }
        }
        (Some(key), None) => {
            let keybinds = &ctx.config.read().unwrap().keybinds;
            if let Some(v) = keybinds.lookup(*key) {
                let output = Output::new()
                    .fg(Color::Yellow)
                    .add(key.to_string())
                    .add(" -> ")
                    .fg(Color::Cyan)
                    .add(v.to_string())
                    .build();
                ctx.queue.status(output);
            } else {
                let output = Output::new()
                    .fg(Color::Red)
                    .add("error: ")
                    .add("unknown command: ")
                    .fg(Color::Cyan)
                    .add(*key)
                    .build();
                ctx.queue.status(output);
            }
        }
        (Some(key), Some(value)) => {
            let keybinds = &mut ctx.config.write().unwrap().keybinds;
            if let Ok(req) = KeyRequest::try_from(*key) {
                if let Some(v) = keybinds.lookup(req) {
                    let next = KeyType::from(*value);
                    let output = Output::new()
                        .fg(Color::Yellow)
                        .add(key.to_string())
                        .fg(Color::Cyan)
                        .add(" ")
                        .add(v.to_string())
                        .add(" -> ")
                        .fg(Color::BrightGreen)
                        .add(next.to_string())
                        .build();
                    ctx.queue.status(output);

                    keybinds.insert(next, req);
                }
            } else {
                let output = Output::new()
                    .fg(Color::Red)
                    .add("error: ")
                    .add("unknown command: ")
                    .fg(Color::Cyan)
                    .add(*key)
                    .build();
                ctx.queue.status(output);
            }
        }
        _ => {}
    }

    ctx.config.read().unwrap().save();
    Ok(())
}

fn rehash_command(ctx: &Context) -> CommandResult {
    match Config::load("riirc.toml") {
        Ok(config) => {
            let conf = &mut *ctx.config.write().unwrap();
            *conf = config
        }
        Err(err) => {
            let output = Output::new()
                .fg(Color::Red)
                .add("error: ")
                .add("cannot load ")
                .fg(Color::BrightWhite)
                .add("config")
                .add(" at ")
                .fg(Color::Yellow)
                .add("riirc.toml")
                .build();
            ctx.queue.status(output);
        }
    };

    Ok(())
}

fn clear_history_command(ctx: &Context) -> CommandResult {
    let (index, _) = ctx.state.current_buffer();
    ctx.queue.request(Request::ClearHistory(index));
    Ok(())
}
