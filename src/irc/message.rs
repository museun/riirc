use crate::irc::command::{Command, Error as CommandError};
use crate::irc::prefix::{Error as PrefixError, Prefix};

#[derive(Debug, PartialEq)]
pub enum Error {
    PrefixError(PrefixError),
    CommandError(CommandError),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Message {
    pub prefix: Option<Prefix>,
    pub command: Command,
}

impl Message {
    pub fn parse(input: &str) -> Result<Self, Error> {
        let prefix = match Prefix::parse(input) {
            Ok((prefix, end)) => Some((prefix, end)),
            Err(PrefixError::MissingLead) => None,
            Err(err) => return Err(Error::PrefixError(err)),
        };

        let input = if prefix.is_some() {
            &input[prefix.as_ref().unwrap().1 + 1..]
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
