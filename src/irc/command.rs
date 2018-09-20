use super::is_valid_nick;

#[derive(Debug, PartialEq)]
pub enum Error {
    MissingCommand,
    MissingData,
    MissingTarget,
    MissingParts,
    InvalidNickname,
    UnknownCommand,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Privmsg {
        target: String,
        data: String,
        is_notice: bool,
    },

    // TODO support multiple channels
    Join {
        channel: String,
        key: Option<String>,
    },
    // TODO support multiple channels
    Part {
        channel: String,
        reason: Option<String>,
    },
    Quit {
        reason: String,
    },
    Nick {
        nickname: String,
    },
    Ping {
        token: String,
    },
    Pong {
        target: String,
    },
    Error {
        message: String,
    },
    // TODO CAP and TAG
    Other {
        command: String,
        params: Vec<String>,
    },
    Reply {
        numeric: u16,
        params: Vec<String>,
    },
}

impl Command {
    pub fn parse(input: &str) -> Result<Self, Error> {
        let pos = input.find(' ').ok_or_else(|| Error::MissingCommand)?;

        let (command, rest) = input.split_at(pos);
        let (command, rest) = (command.trim(), rest.trim());

        let msg = match command {
            "PRIVMSG" | "NOTICE" => {
                let (l, r) = rest.split_at(rest.find(':').ok_or_else(|| Error::MissingData)?);
                let (target, data) = (l.trim(), &r.trim()[1..]);
                if target.is_empty() {
                    return Err(Error::MissingTarget);
                }

                if data.is_empty() {
                    return Err(Error::MissingData);
                }

                // TODO determine if target is a channel
                Command::Privmsg {
                    target: target.into(),
                    data: data.into(),
                    is_notice: command == "NOTICE",
                }
            }

            "JOIN" => {
                if rest.is_empty() {
                    return Err(Error::MissingTarget);
                }

                let mut parts = rest.split(' ');
                let channel = parts
                    .next()
                    .ok_or_else(|| Error::MissingTarget)?
                    .split(',')
                    .next()
                    .unwrap()[1..]
                    .to_owned();
                let key = parts
                    .next()
                    .and_then(|s| s.split(',').next().map(|s| s.to_owned()));

                Command::Join { channel, key }
            }

            "PART" => {
                if rest.is_empty() {
                    return Err(Error::MissingTarget);
                }

                let mut parts = rest.split(' ');
                let channel = parts
                    .next()
                    .ok_or_else(|| Error::MissingTarget)?
                    .split(',')
                    .next()
                    .unwrap()
                    .to_owned();
                let reason = parts.next().map(|s| s.to_owned());

                Command::Part { channel, reason }
            }

            "QUIT" => {
                if rest.get(0..1) != Some(":") {
                    return Err(Error::MissingData);
                }

                Command::Quit {
                    reason: rest[1..].to_owned(),
                }
            }

            "NICK" => {
                if !is_valid_nick(rest) || rest.is_empty() {
                    return Err(Error::InvalidNickname);
                }

                Command::Nick {
                    nickname: rest.to_owned(),
                }
            }

            "PING" => Command::Ping {
                token: rest.to_owned(),
            },

            "PONG" => Command::Pong {
                target: rest.to_owned(),
            },

            "ERROR" => {
                if rest.get(0..1) != Some(":") {
                    return Err(Error::MissingData);
                }
                Command::Error {
                    message: rest[1..].to_owned(),
                }
            }

            command => {
                let params = if let Some(pos) = rest.find(':') {
                    let (l, r) = rest.split_at(pos);
                    let (l, r) = (l.trim(), r.trim());
                    let r = if r.get(0..1) == Some(":") { &r[1..] } else { r }.to_owned();

                    if l.is_empty() {
                        vec![r]
                    } else {
                        let mut v = l.split(' ').map(|s| s.to_owned()).collect::<Vec<_>>();
                        v.push(r);
                        v
                    }
                } else {
                    rest.split(' ').map(|s| s.to_owned()).collect::<Vec<_>>()
                };

                if let Ok(n) = command.parse::<u16>() {
                    Command::Reply { numeric: n, params }
                } else {
                    Command::Other {
                        command: command.to_owned(),
                        params,
                    }
                }
            }
        };

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_privmsg() {
        let input = "PRIVMSG #testchannel :this is a message";
        let command = Command::parse(input);
        assert_eq!(
            command,
            Ok(Command::Privmsg {
                target: "#testchannel".into(),
                data: "this is a message".into(),
                is_notice: false
            })
        );

        let input = "PRIVMSG #testchannel";
        let command = Command::parse(input);
        assert_eq!(command, Err(Error::MissingData));

        let input = "PRIVMSG #testchannel :";
        let command = Command::parse(input);
        assert_eq!(command, Err(Error::MissingData));

        let input = "PRIVMSG :this is a test";
        let command = Command::parse(input);
        assert_eq!(command, Err(Error::MissingTarget));
    }

    #[test]
    fn parse_notice() {
        let input = "NOTICE #testchannel :this is a message";
        let command = Command::parse(input);
        assert_eq!(
            command,
            Ok(Command::Privmsg {
                target: "#testchannel".into(),
                data: "this is a message".into(),
                is_notice: true
            })
        );
    }

    #[test]
    fn parse_join() {
        let inputs = &[
            ("JOIN #test", ("#test", None)),
            ("JOIN #test,&channel", ("#test", None)),
            ("JOIN #test,&channel key1", ("#test", Some("key1"))),
            ("JOIN #test,&channel key1,key2", ("#test", Some("key1"))),
        ];

        for input in inputs {
            let command = Command::parse(input.0);
            assert_eq!(
                command,
                Ok(Command::Join {
                    channel: (input.1).0.into(),
                    key: (input.1).1.map(|s| s.into()),
                })
            );
        }

        let command = Command::parse("JOIN ");
        assert_eq!(command, Err(Error::MissingTarget));
    }

    #[test]
    fn parse_part() {
        let inputs = &[
            ("PART #test", ("#test", None)),
            ("PART #test,&channel", ("#test", None)),
            ("PART #test,&channel bye", ("#test", Some("bye"))),
        ];

        for input in inputs {
            let command = Command::parse(input.0);
            assert_eq!(
                command,
                Ok(Command::Part {
                    channel: (input.1).0.into(),
                    reason: (input.1).1.map(|s| s.into()),
                })
            );
        }

        let command = Command::parse("PART ");
        assert_eq!(command, Err(Error::MissingTarget));
    }

    #[test]
    fn parse_quit() {
        let input = "QUIT :this is a quit message";
        let command = Command::parse(input);
        assert_eq!(
            command,
            Ok(Command::Quit {
                reason: "this is a quit message".into()
            })
        );

        let command = Command::parse("QUIT this is a bad message");
        assert_eq!(command, Err(Error::MissingData));
    }

    #[test]
    fn parse_nick() {
        let input = "NICK test_user";
        let command = Command::parse(input);
        assert_eq!(
            command,
            Ok(Command::Nick {
                nickname: "test_user".into()
            })
        );

        let command = Command::parse("NICK ");
        assert_eq!(command, Err(Error::InvalidNickname));
    }

    #[test]
    fn parse_ping() {
        let inputs = &[("PING test", "test"), ("PING :test", ":test")];
        for input in inputs {
            assert_eq!(
                Command::parse(input.0),
                Ok(Command::Ping {
                    token: input.1.into()
                })
            );
        }
    }

    #[test]
    fn parse_pong() {
        let inputs = &[("PONG test", "test"), ("PONG :test", ":test")];
        for input in inputs {
            assert_eq!(
                Command::parse(input.0),
                Ok(Command::Pong {
                    target: input.1.into()
                })
            );
        }
    }

    #[test]
    fn parse_error() {
        assert_eq!(
            Command::parse("ERROR :test"),
            Ok(Command::Error {
                message: "test".into()
            })
        );

        assert_eq!(Command::parse("ERROR test"), Err(Error::MissingData));
    }

    #[test]
    fn parse_reply() {
        assert_eq!(
            Command::parse("001 :Welcome to the Internet Relay Network test!user@localhost"),
            Ok(Command::Reply {
                numeric: 001,
                params: vec!["Welcome to the Internet Relay Network test!user@localhost".into()]
            })
        );

        assert_eq!(
            Command::parse("312 user irc.localhost :some info"),
            Ok(Command::Reply {
                numeric: 312,
                params: vec!["user", "irc.localhost", "some info"]
                    .into_iter()
                    .map(|s| s.into())
                    .collect()
            })
        );
    }

    #[test]
    fn parse_other() {
        assert_eq!(
            Command::parse("WHOIS eff.org trillian"),
            Ok(Command::Other {
                command: "WHOIS".into(),
                params: vec!["eff.org", "trillian"]
                    .into_iter()
                    .map(|s| s.into())
                    .collect()
            })
        );
        assert_eq!(
            Command::parse("USER guest 0 * :Some user"),
            Ok(Command::Other {
                command: "USER".into(),
                params: vec!["guest", "0", "*", "Some user"]
                    .into_iter()
                    .map(|s| s.into())
                    .collect()
            })
        );
    }
}
