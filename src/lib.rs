#![allow(dead_code)]
#[macro_use]
extern crate log;
extern crate chrono;
extern crate crossbeam_channel;

pub mod irc;
pub mod ui;
pub use self::ui::Gui;

pub mod config;
pub use self::config::Config;
