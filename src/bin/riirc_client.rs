#![allow(dead_code, unused_variables)]

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate pancurses;
extern crate riirc;

mod curses;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let mut ui = curses::Gui::new();

    loop {
        match ui.run() {
            curses::RunState::Exit => return,
            curses::RunState::Continue => continue,
        }
    }
}
