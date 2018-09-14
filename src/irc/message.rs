// why does this need to be FQN?
use crate::irc::command::{Command, Error as CommandError};
use crate::irc::prefix::{Error as PrefixError, Prefix};

use std::io::Write;
type IoResult = ::std::io::Result<()>;

pub fn pass(sink: &mut impl Write, pass: impl AsRef<str>) -> IoResult {
    write!(sink, "PASS {}\r\n", pass.as_ref())
}

pub fn user(sink: &mut impl Write, user: impl AsRef<str>, host: impl AsRef<str>) -> IoResult {
    write!(sink, "USER {} 8 * :{}\r\n", host.as_ref(), user.as_ref())
}

pub fn nick(sink: &mut impl Write, nick: impl AsRef<str>) -> IoResult {
    write!(sink, "NICK {}\r\n", nick.as_ref())
}

pub fn ping(sink: &mut impl Write, token: impl AsRef<str>) -> IoResult {
    write!(sink, "PING {}\r\n", token.as_ref())
}

pub fn pong(sink: &mut impl Write, token: impl AsRef<str>) -> IoResult {
    write!(sink, "PONG {}\r\n", token.as_ref())
}

pub fn join<S, V>(sink: &mut impl Write, channels: V, keys: V) -> IoResult
where
    S: AsRef<str>,
    V: AsRef<[S]>,
{
    macro_rules! as_ref {
        ($e:expr) => {
            $e.as_ref().iter().map(|s| s.as_ref())
        };
    }

    let channels = join_with(as_ref!(channels), ",");
    let keys = join_with(as_ref!(keys), ",");
    write!(sink, "JOIN {} {}\r\n", channels, keys)
}

pub fn part<S, V>(sink: &mut impl Write, channels: V, reason: impl AsRef<str>) -> IoResult
where
    S: AsRef<str>,
    V: AsRef<[S]>,
{
    macro_rules! as_ref {
        ($e:expr) => {
            $e.as_ref().iter().map(|s| s.as_ref())
        };
    }
    let channels = join_with(as_ref!(channels), ",");
    write!(sink, "PART {} :{}\r\n", channels, reason.as_ref())
}

pub fn privmsg(sink: &mut impl Write, target: impl AsRef<str>, data: impl AsRef<str>) -> IoResult {
    let target = target.as_ref();
    let data = data.as_ref();
    write!(sink, "PRIVMSG {} :{}\r\n", target, data)
}

fn join_with<'a>(i: impl Iterator<Item = &'a str>, s: &str) -> String {
    let s = i.fold(String::new(), |mut a, c| {
        a.push_str(c);
        a.push_str(s);
        a
    });
    s.trim_right_matches(',').to_owned()
}

#[derive(Debug, PartialEq)]
pub enum Error {
    PrefixError(PrefixError),
    CommandError(CommandError),
}

#[derive(Debug, PartialEq)]
pub struct Message<'a> {
    prefix: Option<Prefix<'a>>,
    command: Command<'a>,
}

impl<'a> Message<'a> {
    pub fn parse(input: &'a str) -> Result<Self, Error> {
        let prefix = match Prefix::parse(input) {
            Err(PrefixError::MissingLead) => None,
            Err(err) => return Err(Error::PrefixError(err)),
            Ok((prefix, end)) => Some((prefix, end)),
        };

        let input = if prefix.is_some() {
            let end = prefix.as_ref().unwrap().1;
            &input[end + 1..]
        } else {
            input
        };

        Ok(Message {
            command: Command::parse(&input).map_err(Error::CommandError)?,
            prefix: prefix.map(|(p, _)| p),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO more comprehensive tests
    #[test]
    fn parse_message() {
        let inputs = &[
            ":tmi.twitch.tv CAP * LS :twitch.tv/tags twitch.tv/commands twitch.tv/membership",
            ":tmi.twitch.tv 001 museun :Welcome, GLHF!",
            ":tmi.twitch.tv 002 museun :Your host is tmi.twitch.tv",
            ":tmi.twitch.tv 003 museun :This server is rather new",
            ":tmi.twitch.tv 004 museun :-",
            ":tmi.twitch.tv 375 museun :-",
            ":tmi.twitch.tv 372 museun :You are in a maze of twisty passages, all alike.",
            ":tmi.twitch.tv 376 museun :>",
            ":tmi.twitch.tv CAP * ACK :twitch.tv/membership",
            ":museun!museun@museun.tmi.twitch.tv JOIN #museun",
            ":museun.tmi.twitch.tv 353 museun = #museun :museun",
            ":museun.tmi.twitch.tv 366 museun #museun :End of /NAMES list",
            ":tmi.twitch.tv 421 museun WHO :Unknown command",
        ];

        for input in inputs {
            let msg = Message::parse(input);
            assert!(msg.is_ok());
        }
    }
}
