#![allow(dead_code, unused_variables)]
#![feature(try_from)]

#[macro_use]
extern crate log;
extern crate env_logger;

mod curses;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let config = match curses::Config::load("riirc.toml") {
        Ok(config) => config,
        Err(err) => {
            curses::Config::default().save();
            info!("wrote default config to: riirc.toml");
            return;
        }
    };

    curses::Gui::new(config).run();
}
