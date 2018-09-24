#![allow(dead_code)]
#[macro_use]
extern crate log;
extern crate chrono;
extern crate crossbeam_channel;

#[macro_export]
macro_rules! import {
    ($($name:ident),+) => {
       $(
            mod $name;
            #[allow(unused_imports)]
            use self::$name::*;
       )*
    };
}

mod config;
mod irc;
mod ui;

pub use self::config::Config;
pub use self::ui::Gui;
