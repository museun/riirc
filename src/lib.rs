#[macro_use]
extern crate log;
extern crate crossbeam_channel;

pub mod irc;
pub use self::irc::*;
