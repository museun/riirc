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

#[derive(Debug, PartialEq)]
pub enum Command<'a> {
    Privmsg {
        target: &'a str,
        data: &'a str,
        is_notice: bool,
    },
    Join {
        target: Vec<&'a str>,
        key: Vec<&'a str>,
    },
    Part {
        target: Vec<&'a str>,
        reason: Option<&'a str>,
    },
    Quit {
        reason: &'a str,
    },
    Nick {
        nickname: &'a str,
    },
    Ping {
        token: &'a str,
    },
    Pong {
        target: &'a str,
    },
    Error {
        message: &'a str,
    },
    // TODO CAP and TAG
    Other {
        command: &'a str,
        params: Vec<&'a str>,
    },
    Reply {
        numeric: u16,
        params: Vec<&'a str>,
    },
}

impl<'a> Command<'a> {
    pub fn parse(input: &'a str) -> Result<Self, Error> {
        let pos = input.find(' ').ok_or_else(|| Error::MissingCommand)?;

        fn get_data(s: &str) -> Result<(&str, &str), Error> {
            let (l, r) = s.split_at(s.find(':').ok_or_else(|| Error::MissingData)?);
            let (l, r) = (l.trim(), &r.trim()[1..]);
            if l.is_empty() {
                return Err(Error::MissingTarget);
            }

            if r.is_empty() {
                return Err(Error::MissingData);
            }
            Ok((l, r))
        }

        let (command, rest) = input.split_at(pos);
        let (command, rest) = (command.trim(), rest.trim());

        match command {
            "PRIVMSG" | "NOTICE" => {
                let (target, data) = get_data(rest)?;
                Ok(Command::Privmsg {
                    target,
                    data,
                    is_notice: command == "NOTICE",
                })
            }
            "JOIN" => {
                if rest.is_empty() {
                    return Err(Error::MissingTarget);
                }

                let mut parts = rest.split(' ');
                let target = match parts.next() {
                    Some(target) => target.split(',').collect::<Vec<_>>(),
                    None => return Err(Error::MissingTarget),
                };
                let key = match parts.next() {
                    Some(key) => key.split(',').collect::<Vec<_>>(),
                    None => vec![],
                };

                Ok(Command::Join { target, key })
            }
            "PART" => {
                if rest.is_empty() {
                    return Err(Error::MissingTarget);
                }

                let mut parts = rest.split(' ');
                let target = match parts.next() {
                    Some(target) => target.split(',').collect::<Vec<_>>(),
                    None => return Err(Error::MissingTarget),
                };

                let reason = parts.next();
                Ok(Command::Part { target, reason })
            }
            "QUIT" => {
                if rest.get(0..1) != Some(":") {
                    return Err(Error::MissingData);
                }

                Ok(Command::Quit { reason: &rest[1..] })
            }
            "NICK" => {
                if !is_valid_nick(rest) || rest.is_empty() {
                    return Err(Error::InvalidNickname);
                }

                Ok(Command::Nick { nickname: rest })
            }
            "PING" => Ok(Command::Ping { token: rest }),
            "PONG" => Ok(Command::Pong { target: rest }),
            "ERROR" => {
                if rest.get(0..1) != Some(":") {
                    return Err(Error::MissingData);
                }
                Ok(Command::Error {
                    message: &rest[1..],
                })
            }

            command => {
                let params = if let Some(pos) = rest.find(':') {
                    let (l, r) = rest.split_at(pos);
                    let (l, r) = (l.trim(), r.trim());
                    let r = if r.get(0..1) == Some(":") { &r[1..] } else { r };

                    if l.is_empty() {
                        vec![r]
                    } else {
                        let mut v = l.split(' ').collect::<Vec<_>>();
                        v.push(r);
                        v
                    }
                } else {
                    rest.split(' ').collect::<Vec<_>>()
                };

                if let Ok(n) = command.parse::<u16>() {
                    Ok(Command::Reply { numeric: n, params })
                } else {
                    Ok(Command::Other { command, params })
                }
            }
        }
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
                target: "#testchannel",
                data: "this is a message",
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
                target: "#testchannel",
                data: "this is a message",
                is_notice: true
            })
        );
    }

    #[test]
    fn parse_join() {
        let inputs = &[
            (
                "JOIN #testchannel", // stop
                (vec!["#testchannel"], vec![]),
            ),
            (
                "JOIN #testchannel,&channel",
                (vec!["#testchannel", "&channel"], vec![]),
            ),
            (
                "JOIN #testchannel,&channel key1",
                (vec!["#testchannel", "&channel"], vec!["key1"]),
            ),
            (
                "JOIN #testchannel,&channel key1,key2",
                (vec!["#testchannel", "&channel"], vec!["key1", "key2"]),
            ),
        ];

        for input in inputs {
            let command = Command::parse(input.0);
            assert_eq!(
                command,
                Ok(Command::Join {
                    target: (input.1).0.clone(),
                    key: (input.1).1.clone(),
                })
            );
        }

        let command = Command::parse("JOIN ");
        assert_eq!(command, Err(Error::MissingTarget));
    }

    #[test]
    fn parse_part() {
        let inputs = &[
            (
                "PART #testchannel", // stop
                (vec!["#testchannel"], None),
            ),
            (
                "PART #testchannel,&channel",
                (vec!["#testchannel", "&channel"], None),
            ),
            (
                "PART #testchannel,&channel bye",
                (vec!["#testchannel", "&channel"], Some("bye")),
            ),
        ];

        for input in inputs {
            let command = Command::parse(input.0);
            assert_eq!(
                command,
                Ok(Command::Part {
                    target: (input.1).0.clone(),
                    reason: (input.1).1,
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
                reason: "this is a quit message"
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
                nickname: "test_user"
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
                Ok(Command::Ping { token: input.1 })
            );
        }
    }

    #[test]
    fn parse_pong() {
        let inputs = &[("PONG test", "test"), ("PONG :test", ":test")];
        for input in inputs {
            assert_eq!(
                Command::parse(input.0),
                Ok(Command::Pong { target: input.1 })
            );
        }
    }

    #[test]
    fn parse_error() {
        assert_eq!(
            Command::parse("ERROR :test"),
            Ok(Command::Error { message: "test" })
        );

        assert_eq!(Command::parse("ERROR test"), Err(Error::MissingData));
    }

    #[test]
    fn parse_reply() {
        assert_eq!(
            Command::parse("001 :Welcome to the Internet Relay Network test!user@localhost"),
            Ok(Command::Reply {
                numeric: 001,
                params: vec!["Welcome to the Internet Relay Network test!user@localhost"]
            })
        );

        assert_eq!(
            Command::parse("312 user irc.localhost :some info"),
            Ok(Command::Reply {
                numeric: 312,
                params: vec!["user", "irc.localhost", "some info"]
            })
        );
    }

    #[test]
    fn parse_other() {
        assert_eq!(
            Command::parse("WHOIS eff.org trillian"),
            Ok(Command::Other {
                command: "WHOIS",
                params: vec!["eff.org", "trillian"]
            })
        );
        assert_eq!(
            Command::parse("USER guest 0 * :Some user"),
            Ok(Command::Other {
                command: "USER",
                params: vec!["guest", "0", "*", "Some user"]
            })
        );
    }
}
