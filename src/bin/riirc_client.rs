#![allow(dead_code, unused_variables)]
#![feature(try_from)]

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate pancurses;
extern crate toml_document;

extern crate riirc;

mod curses;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let config = match curses::Config::load("riirc.toml") {
        Ok(config) => config,
        Err(err) => {
            let _ = curses::Config::default();
            info!("wrote default config to: riirc.toml");
            return;
        }
    };

    let mut ui = curses::Gui::new(config);
    loop {
        match ui.run() {
            curses::RunState::Exit => return,
            curses::RunState::Continue => continue,
        }
    }
}
