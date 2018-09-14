use std::fmt;

#[derive(PartialEq, Debug)]
pub enum Error {
    MissingLead,
    MissingPrefix,
    MissingHost,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MissingLead => write!(f, "missing lead colon"),
            Error::MissingPrefix => write!(f, "missing entire prefix"),
            Error::MissingHost => write!(f, "missing @ separator for user host"),
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum Prefix<'a> {
    User {
        nick: &'a str,
        user: &'a str,
        host: &'a str,
    },
    Server {
        host: &'a str,
    },
}

impl<'a> Prefix<'a> {
    pub fn parse(input: &'a str) -> Result<(Self, usize), Error> {
        if !input.starts_with(':') {
            return Err(Error::MissingLead);
        }

        let end = input.find(' ').ok_or_else(|| Error::MissingPrefix)?;
        let s = input[1..end].trim();
        match s.find('!') {
            Some(pos) => {
                let nick = &s[..pos];
                let at = s.find('@').ok_or_else(|| Error::MissingHost)?;
                let user = &s[pos + 1..at];
                let host = &s[at + 1..];
                Ok((Prefix::User { nick, user, host }, end))
            }
            None => Ok((Prefix::Server { host: s }, end)),
        }
    }
}

impl<'a> fmt::Debug for Prefix<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Prefix::User {
                ref nick,
                ref user,
                ref host,
            } => write!(f, "{}!{}@{}", nick, user, host),
            Prefix::Server { ref host } => write!(f, "{}", host),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_user() {
        let input = ":testuser{12}!user~@local.host ";
        let prefix = Prefix::parse(input);
        match prefix {
            Ok((Prefix::User { nick, user, host }, _)) => {
                assert_eq!(nick, "testuser{12}");
                assert_eq!(user, "user~");
                assert_eq!(host, "local.host");
            }
            Ok((Prefix::Server { .. }, _)) => panic!("parsed server prefix"),
            Err(err) => panic!("failed to parse user prefix: {}", err),
        }

        let input = "testuser!user@host ";
        let prefix = Prefix::parse(input);
        assert_eq!(prefix, Err(Error::MissingLead));

        let input = ":testuser!user ";
        let prefix = Prefix::parse(input);
        assert_eq!(prefix, Err(Error::MissingHost));

        let input = ":invalidmessage";
        let prefix = Prefix::parse(input);
        assert_eq!(prefix, Err(Error::MissingPrefix));
    }

    #[test]
    fn parse_server() {
        let input = ":irc.test.server ";
        let prefix = Prefix::parse(input);
        match prefix {
            Ok((Prefix::Server { host }, _)) => {
                assert_eq!(host, "irc.test.server");
            }
            Ok((Prefix::User { .. }, _)) => panic!("parsed user prefix"),
            Err(err) => panic!("failed to parse server prefix: {}", err),
        }
    }
}
