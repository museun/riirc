extern crate pancurses;
extern crate riirc;
extern crate toml_document;

use std::sync::Arc;
type CWindow = Arc<pancurses::Window>;

mod command;
use self::command::Processor;

mod request;
use self::request::*;

mod inputbuffer;
use self::inputbuffer::InputBuffer;

mod window;
use self::window::{InnerWindow, Window, WindowBuilder};

mod inputwindow;
use self::inputwindow::{InputWindow, ReadType};

mod outputwindow;
use self::outputwindow::OutputWindow;

mod nicklistwindow;
use self::nicklistwindow::NicklistWindow;

mod buffer;
use self::buffer::Buffer;

mod state;
use self::state::State;

mod messagequeue;
use self::messagequeue::*;

mod colors;
use self::colors::*;

mod output;
use self::output::*;

pub mod gui;
pub use self::gui::Gui;

mod keybinds;
pub use self::keybinds::*;

mod config;
pub use self::config::*;

pub mod boundedset;
