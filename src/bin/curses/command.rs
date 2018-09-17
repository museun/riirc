use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use riirc::IrcClient;

type Command = fn(&Arc<State>, &Arc<MessageQueue>, &[&str]);

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
        self.map.insert("/connect", connect_command);
        self.map.insert("/quit", quit_command);
        self.map.insert("/clear", clear_command);
        self.map.insert("/join", join_command);
        self.map.insert("/part", part_command);
        self.map.insert("/buffer", buffer_command);
        self.map.insert("/buffers", list_buffer_command);
    }

    pub fn handle(&mut self, input: &str) {
        let input = input.to_string();
        let mut input = input.split(' ');
        let query = input.next().unwrap();
        if !self.map.contains_key(query) {
            self.queue.status(format!("unknown command: {}", query));
            return;
        }

        let parts = input.collect::<Vec<_>>();
        let func = self.map[&query];
        func(&Arc::clone(&self.state), &Arc::clone(&self.queue), &parts)
    }
}

fn echo_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    for part in parts {
        queue.status(part.to_string());
    }
}

fn connect_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    {
        if state.client().is_some() {
            queue.status("already connected");
            return;
        }
    }

    let client = riirc::Client::new("localhost:6667").expect("connect to localhost");
    let errors = client.run();
    // do something with the errors

    client.nick("test");
    client.user("test", "test name");

    {
        state.set_client(client, errors);
    }
}

fn quit_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    if !state.assume_connected() {
        return;
    }

    let msg = if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    };

    queue.push(Request::Quit(msg));
}

fn join_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    if !state.assume_connected() {
        return;
    }

    if parts.is_empty() {
        queue.status("try: /join <chan>");
        return;
    }

    // TODO make this actually work on multiple channerls + keys
    queue.push(Request::Join(parts[0].to_owned()));
}

fn part_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    if !state.assume_connected() {
        return;
    }

    let (status, buf) = state.at_status_buffer();
    if status {
        queue.status("cannot /part in a *window");
        return;
    }

    let ch = if parts.is_empty() {
        buf.name().to_string()
    } else {
        parts[0].to_string()
    };

    queue.push(Request::Part(ch));
}

fn clear_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    queue.push(Request::Clear(true));
}

fn buffer_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    if parts.is_empty() {
        queue.status("try /buffer N");
        return;
    }

    let buf = match parts[0].parse::<usize>() {
        Ok(n) => n,
        Err(_err) => {
            queue.status("try /buffer N (a number this time)");
            return;
        }
    };

    queue.push(Request::SwitchBuffer(buf))
}

fn list_buffer_command(state: &Arc<State>, queue: &Arc<MessageQueue>, parts: &[&str]) {
    let buffers = state.buffers();

    for (n, buffer) in buffers.iter().enumerate() {
        queue.status(format!("buffer #{}: {}", n, buffer.name()))
    }
}
