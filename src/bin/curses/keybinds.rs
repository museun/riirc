use super::{inputbuffer, messagequeue};
use std::convert::{TryFrom, TryInto};
use std::fmt;

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct KeyType(String);

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Key> for KeyType {
    fn from(k: Key) -> Self {
        use self::Mod::*;

        let mut buf = String::new();
        match k.modifier {
            Ctrl => buf.push_str("C-"),
            Alt => buf.push_str("A-"),
            Shift => buf.push_str("S-"),
            None => {}
        };

        match k.kind {
            KeyKind::Num(n) => buf.push_str(&format!("{}", n)),
            KeyKind::Char(ch) | KeyKind::Other(ch) => buf.push_str(&format!("{}", ch)),
            _ => {}
        }

        KeyType(buf)
    }
}

impl From<String> for KeyType {
    fn from(s: String) -> Self {
        KeyType(s)
    }
}

impl<'a> From<&'a str> for KeyType {
    fn from(s: &'a str) -> Self {
        KeyType(s.into())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Keybinds(Vec<(KeyRequest, KeyType)>);

impl Keybinds {
    pub fn get(&self, key: &KeyType) -> Option<&KeyRequest> {
        if key.0.is_empty() {
            return None;
        }

        for (v, k) in &self.0 {
            if k == key {
                return Some(v);
            }
        }
        None
    }

    pub fn insert(&mut self, key: impl Into<KeyType>, req: KeyRequest) {
        let key = key.into();
        for (v, k) in &mut self.0 {
            if *v == req {
                *k = key.clone()
            }
        }
    }

    pub fn lookup<K>(&self, req: K) -> Option<&KeyType>
    where
        K: TryInto<KeyRequest>,
    {
        if let Ok(req) = req.try_into() {
            if let Some(pos) = self.iter().position(|(r, _)| *r == req) {
                return self.0.get(pos).map(|(_, v)| v);
            }
        }
        None
    }

    pub fn lookup_key(&self, key: impl Into<KeyType>) -> Option<&KeyRequest> {
        let key = key.into();
        if let Some(pos) = self.iter().position(|(_, k)| *k == key) {
            return self.0.get(pos).map(|(k, _)| k);
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = &(KeyRequest, KeyType)> {
        self.0.iter()
    }
}

impl Default for Keybinds {
    fn default() -> Self {
        let map = vec![
            (KeyRequest::Clear, "C-l".into()),
            (KeyRequest::RecallBackward, "A-p".into()),
            (KeyRequest::RecallForward, "A-n".into()),
            //
            (KeyRequest::ToggleNickList, "A-k".into()),
            //
            (KeyRequest::MoveForward, "C-f".into()),
            (KeyRequest::MoveBackward, "C-b".into()),
            (KeyRequest::MoveForwardWord, "A-f".into()),
            (KeyRequest::MoveBackwardWord, "A-b".into()),
            (KeyRequest::MoveStart, "C-a".into()),
            (KeyRequest::MoveEnd, "C-e".into()),
            //
            (KeyRequest::DeleteForward, "C-d".into()),
            (KeyRequest::DeleteForwardWord, "A-d".into()),
            (KeyRequest::DeleteBackwardWord, "C-w".into()),
            (KeyRequest::DeleteBackward, "A-w".into()),
            (KeyRequest::DeleteStart, "C-u".into()),
            (KeyRequest::DeleteEnd, "C-k".into()),
            //
            (KeyRequest::SwapCaseForward, "".into()),
            (KeyRequest::SwapCaseForwardWord, "A-u".into()),
            (KeyRequest::SwapCaseBackwardWord, "".into()),
            (KeyRequest::SwapCaseBackward, "".into()),
            (KeyRequest::SwapCaseStart, "".into()),
            (KeyRequest::SwapCaseEnd, "".into()),
            //
            (KeyRequest::PrevBuffer, "C-p".into()),
            (KeyRequest::NextBuffer, "C-n".into()),
            //
            (KeyRequest::SwitchBuffer0, "C-0".into()),
            (KeyRequest::SwitchBuffer1, "C-1".into()),
            (KeyRequest::SwitchBuffer2, "C-2".into()),
            (KeyRequest::SwitchBuffer3, "C-3".into()),
            (KeyRequest::SwitchBuffer4, "C-4".into()),
            (KeyRequest::SwitchBuffer5, "C-5".into()),
            (KeyRequest::SwitchBuffer6, "C-6".into()),
            (KeyRequest::SwitchBuffer7, "C-7".into()),
            (KeyRequest::SwitchBuffer8, "C-8".into()),
            (KeyRequest::SwitchBuffer9, "C-9".into()),
        ];
        Self { 0: map }
    }
}

// fully enumerated so they can show up in the config easier
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum KeyRequest {
    Clear,

    ToggleNickList,

    PrevBuffer,
    NextBuffer,

    RecallBackward,
    RecallForward,

    MoveForward,
    MoveBackward,
    MoveForwardWord,
    MoveBackwardWord,
    MoveStart,
    MoveEnd,

    DeleteForward,
    DeleteBackward,
    DeleteForwardWord,
    DeleteBackwardWord,
    DeleteStart,
    DeleteEnd,

    SwapCaseForward,
    SwapCaseBackward,
    SwapCaseForwardWord,
    SwapCaseBackwardWord,
    SwapCaseStart,
    SwapCaseEnd,

    SwitchBuffer0,
    SwitchBuffer1,
    SwitchBuffer2,
    SwitchBuffer3,
    SwitchBuffer4,
    SwitchBuffer5,
    SwitchBuffer6,
    SwitchBuffer7,
    SwitchBuffer8,
    SwitchBuffer9,
}

impl fmt::Display for KeyRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:?}", self);
        let mut buf = String::new();

        for (i, ch) in s.chars().enumerate() {
            if i > 0 && (ch.is_numeric() || ch.is_uppercase()) {
                buf.push('_');
                buf.push_str(&s[i..=i].to_ascii_lowercase());
            } else {
                buf.push(ch.to_ascii_lowercase());
            }
        }

        write!(f, "{}", buf)
    }
}

impl TryFrom<String> for KeyRequest {
    type Error = ();
    fn try_from(s: String) -> Result<Self, Self::Error> {
        KeyRequest::try_from(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for KeyRequest {
    type Error = ();
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        use self::KeyRequest::*;
        fn unsnakecase(s: &str) -> String {
            let mut buf = String::new();
            let mut prev = false;
            for (i, c) in s.chars().enumerate() {
                if i == 0 || prev {
                    buf.push(c.to_ascii_uppercase());
                    prev = false;
                    continue;
                }
                if c == '_' {
                    prev = true;
                    continue;
                }
                buf.push(c)
            }
            buf
        }

        let res = match unsnakecase(&s).as_str() {
            "Clear" => Clear,
            "RecallBackward" => RecallBackward,
            "RecallForward" => RecallForward,
            "ToggleNickList" => ToggleNickList,
            "PrevBuffer" => PrevBuffer,
            "NextBuffer" => NextBuffer,
            "MoveForward" => MoveForward,
            "MoveBackward" => MoveBackward,
            "MoveForwardWord" => MoveForwardWord,
            "MoveBackwardWord" => MoveBackwardWord,
            "MoveStart" => MoveStart,
            "MoveEnd" => MoveEnd,
            "DeleteForward" => DeleteForward,
            "DeleteBackward" => DeleteBackward,
            "DeleteForwardWord" => DeleteForwardWord,
            "DeleteBackwardWord" => DeleteBackwardWord,
            "DeleteStart" => DeleteStart,
            "DeleteEnd" => DeleteEnd,
            "SwapCaseForward" => SwapCaseForward,
            "SwapCaseBackward" => SwapCaseBackward,
            "SwapCaseForwardWord" => SwapCaseForwardWord,
            "SwapCaseBackwardWord" => SwapCaseBackwardWord,
            "SwapCaseStart" => SwapCaseStart,
            "SwapCaseEnd" => SwapCaseEnd,
            "SwitchBuffer0" => SwitchBuffer0,
            "SwitchBuffer1" => SwitchBuffer1,
            "SwitchBuffer2" => SwitchBuffer2,
            "SwitchBuffer3" => SwitchBuffer3,
            "SwitchBuffer4" => SwitchBuffer4,
            "SwitchBuffer5" => SwitchBuffer5,
            "SwitchBuffer6" => SwitchBuffer6,
            "SwitchBuffer7" => SwitchBuffer7,
            "SwitchBuffer8" => SwitchBuffer8,
            "SwitchBuffer9" => SwitchBuffer9,
            _ => return Err(()),
        };
        Ok(res)
    }
}

impl TryFrom<KeyRequest> for messagequeue::Request {
    type Error = ();
    fn try_from(kr: KeyRequest) -> Result<messagequeue::Request, Self::Error> {
        use self::KeyRequest::*;
        use super::messagequeue::Request;

        // for msg queue requests
        let res = match kr {
            Clear => Request::Clear(true),

            ToggleNickList => Request::ToggleNickList,

            PrevBuffer => Request::PrevBuffer,
            NextBuffer => Request::NextBuffer,

            SwitchBuffer0 => Request::SwitchBuffer(0),
            SwitchBuffer1 => Request::SwitchBuffer(1),
            SwitchBuffer2 => Request::SwitchBuffer(2),
            SwitchBuffer3 => Request::SwitchBuffer(3),
            SwitchBuffer4 => Request::SwitchBuffer(4),
            SwitchBuffer5 => Request::SwitchBuffer(5),
            SwitchBuffer6 => Request::SwitchBuffer(6),
            SwitchBuffer7 => Request::SwitchBuffer(7),
            SwitchBuffer8 => Request::SwitchBuffer(8),
            SwitchBuffer9 => Request::SwitchBuffer(9),
            _ => return Err(()),
        };

        Ok(res)
    }
}

impl TryFrom<KeyRequest> for inputbuffer::Command {
    type Error = ();

    fn try_from(kr: KeyRequest) -> Result<inputbuffer::Command, Self::Error> {
        use self::KeyRequest::*;
        use super::inputbuffer::Command::*;
        use super::inputbuffer::Move::*;

        // for input commands
        let res = match kr {
            RecallForward => Recall(Forward),
            RecallBackward => Recall(Backward),

            MoveForward => Move(Forward),
            MoveBackward => Move(Backward),
            MoveForwardWord => Move(ForwardWord),
            MoveBackwardWord => Move(BackwardWord),
            MoveStart => Move(StartOfLine),
            MoveEnd => Move(EndOfLine),

            DeleteForward => Delete(Forward),
            DeleteBackward => Delete(Backward),
            DeleteForwardWord => Delete(ForwardWord),
            DeleteBackwardWord => Delete(BackwardWord),
            DeleteStart => Delete(StartOfLine),
            DeleteEnd => Delete(EndOfLine),

            SwapCaseForward => SwapCase(Forward),
            SwapCaseBackward => SwapCase(Backward),
            SwapCaseForwardWord => SwapCase(ForwardWord),
            SwapCaseBackwardWord => SwapCase(BackwardWord),
            SwapCaseStart => SwapCase(StartOfLine),
            SwapCaseEnd => SwapCase(EndOfLine),
            _ => return Err(()),
        };

        Ok(res)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mod {
    Ctrl,
    Alt,
    Shift,
    None,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Key {
    pub modifier: Mod,
    pub kind: KeyKind,
}

impl Key {
    pub fn parse(v: u16) -> Option<Self> {
        let mut modifier = Mod::None;
        let kind = KeyKind::new(v, &mut modifier)?;
        Some(Key { modifier, kind })
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum KeyKind {
    Backspace,
    Enter,
    Tab,

    Num(usize),

    Char(char),
    Other(char),
}

#[rustfmt::skip]
impl KeyKind {
    pub fn new(v: u16, m: &mut Mod) -> Option<KeyKind> {

        use self::KeyKind::*;
        let key = match v {
            0xECF8 => { *m = Mod::Alt; Backspace }
            0x007F => { *m = Mod::Ctrl; Backspace }
            0x0008 => { *m = Mod::None; Backspace }

            0xECEE => { *m = Mod::Alt; Enter }
            0xED11 => { *m = Mod::Ctrl; Enter }
            0x000A => { *m = Mod::None; Enter }

            // Alt-tab won't work for .. obvious reasons
            0xECE2 => { *m = Mod::Ctrl; Tab }
            // this is a fake key
            0xECED => { debug!("fake key, maybe not a tab");
                        *m = Mod::Shift; Tab }
            0x0009 => { *m = Mod::None; Tab }
 
            0xEC97...0xECA0 => { *m = Mod::Alt; Num((v - 0xEC97) as usize) }
            0xED37...0xED40 => { *m = Mod::Ctrl; Num((v - 0xED37) as usize) }

            0xECA1...0xECBA => { *m = Mod::Alt; Char(((v as u8) - 0x40) as char) },
            0x0001...0x001A if v != 0x000A => { *m = Mod::Ctrl; Char(((v as u8) + 0x60) as char) },
            0x0061...0x007A | 0x0040 => { *m = Mod::None; Char((v as u8) as char) },
            0x0041...0x005A => { *m = Mod::Shift; Char(((v as u8) + 0x20) as char) },

            _ => { *m = Mod::None; Other((v as u8) as char) },
        };

        eprintln!("0x{:04X} | {:>6} | {:?}, {:?}", v, v, m, key);

        Some(key)
    }
}
