use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use riirc::IrcClient;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidArgument(&'static str),
    InvalidBuffer(&'static str),
    AlreadyConnected,
    NotConnected,
    ForceExit,
}

impl Error {
    fn report_error(&self, ctx: &Context) {
        use self::Error::*;

        match &self {
            InvalidArgument(s) | InvalidBuffer(s) => ctx.queue.status(s),
            AlreadyConnected => ctx.queue.status("already connected"),
            NotConnected => ctx.queue.status("not connected"),
            _ => {}
        }
    }
}

// TODO make a help system
// TODO TODO deserialize it from a file

type CommandResult = Result<(), Error>;
type Command = fn(&Context) -> CommandResult;

struct Context<'a> {
    state: Arc<State>,
    queue: Arc<MessageQueue>,
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
    }

    pub fn handle(&mut self, input: &str) -> CommandResult {
        if input.is_empty() {
            Ok(())?;
        }

        if !input.starts_with('/') {
            self.state.send_line(input);
            Ok(())?;
        }

        let input = input.to_string();
        let mut input = input.split(' ');
        let query = input.next().unwrap();
        if !self.map.contains_key(query) {
            self.queue.status(format!("unknown command: {}", query));
            Ok(())?;
        }

        let parts = input.collect::<Vec<_>>();
        let func = self.map[&query];
        let ctx = Context {
            state: Arc::clone(&self.state),
            queue: Arc::clone(&self.queue),
            parts: &parts,
        };
        match func(&ctx) {
            Err(ref err) if *err != Error::ForceExit => err.report_error(&ctx),
            Err(err) => Err(err)?,
            _ => {}
        };

        Ok(())
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
        ctx.queue.status(part.to_string());
    }
    Ok(())
}

fn connect_command(ctx: &Context) -> CommandResult {
    if ctx.state.client().is_some() {
        Err(Error::AlreadyConnected)?;
    };

    // TODO get this from a config file
    let client = riirc::Client::new("localhost:6667").expect("connect to localhost");
    client.nick("test");
    client.user("test", "test name");

    let errors = client.run();
    ctx.state.set_client(client, errors);

    Ok(())
}

fn quit_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;

    let msg = if ctx.parts.is_empty() {
        None
    } else {
        Some(ctx.parts.join(" "))
    };

    ctx.queue.push(Request::Quit(msg));
    Ok(())
}

fn join_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;
    assume_args(&ctx, "try: /join <chan>")?;

    // TODO make this actually work on multiple channerls + keys
    ctx.queue.push(Request::Join(ctx.parts[0].to_owned()));
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

    ctx.queue.push(Request::Part(ch));
    Ok(())
}

fn clear_command(ctx: &Context) -> CommandResult {
    ctx.queue.push(Request::Clear(true));
    Ok(())
}

fn buffer_command(ctx: &Context) -> CommandResult {
    assume_args(&ctx, "try: /buffer N")?;

    let buf = ctx.parts[0]
        .parse::<usize>()
        .map_err(|_e| Error::InvalidArgument("try: /buffer N (a number this time)"))?;

    ctx.queue.push(Request::SwitchBuffer(buf));
    Ok(())
}

fn list_buffers_command(ctx: &Context) -> CommandResult {
    let buffers = ctx.state.buffers();
    let len = ctx.state.buffers_len() - 1;

    let mut output = String::from("buffers: ");
    for (n, buffer) in buffers.iter().enumerate() {
        if n < len {
            output.push_str(&format!("{}:{}, ", n, buffer.name()))
        } else {
            output.push_str(&format!("{}:{}", n, buffer.name()))
        }
    }

    ctx.queue.status(output);
    Ok(())
}
