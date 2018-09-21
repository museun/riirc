use super::*;

#[derive(Debug, PartialEq)]
pub enum Request {
    Clear(bool),
    Join(String),
    Part(String),
    Quit(Option<String>),

    ToggleNickList,
    ClearHistory(usize),

    SwitchBuffer(usize),
    NextBuffer,
    PrevBuffer,

    Queue(usize, Output),  // buffer index
    Target(usize, Output), // buffer index
}
