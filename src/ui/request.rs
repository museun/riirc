use super::output::Output;

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

#[derive(Debug, PartialEq)]
pub enum Command {
    Delete(Move),
    SwapCase(Move),
    Insert(usize, char),
    Append(char),
    Move(Move),

    // these aren't really movements
    Recall(Move),
}

#[derive(Debug, PartialEq)]
pub enum Move {
    EndOfLine,
    StartOfLine,
    ForwardWord,
    Forward,
    BackwardWord,
    Backward,
    Exact(usize),
}
