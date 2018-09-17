use super::client::{Client, Inner};

pub trait IrcClient {
    fn privmsg(&self, target: impl AsRef<str>, data: impl AsRef<str>) {
        self.write(&format!("PRIVMSG {} :{}\r\n", target.as_ref(), data.as_ref()).as_bytes())
    }

    fn pass(&self, pass: impl AsRef<str>) {
        self.write(&format!("PASS {}\r\n", pass.as_ref()).as_bytes())
    }

    fn user(&self, real: impl AsRef<str>, user: impl AsRef<str>) {
        self.write(&format!("USER {} 8 * :{}\r\n", user.as_ref(), real.as_ref()).as_bytes())
    }

    fn nick(&self, nick: impl AsRef<str>) {
        self.write(&format!("NICK {}\r\n", nick.as_ref()).as_bytes())
    }

    fn ping(&self, token: impl AsRef<str>) {
        self.write(&format!("PING {}\r\n", token.as_ref()).as_bytes())
    }

    fn pong(&self, token: impl AsRef<str>) {
        self.write(&format!("PONG {}\r\n", token.as_ref()).as_bytes())
    }

    fn quit(&self, msg: Option<String>) {
        self.write(
            &format!(
                "QUIT :{}\r\n",
                if let Some(msg) = msg {
                    msg
                } else {
                    "bye".to_string()
                }
            ).as_bytes(),
        );

        self.close();
    }

    fn join(&self, channel: impl AsRef<str>, key: Option<&str>) {
        let channel = channel.as_ref();
        let msg = if key.is_some() {
            format!("JOIN {} {}\r\n", channel, key.unwrap())
        } else {
            format!("JOIN {}\r\n", channel)
        };

        self.write(msg.as_bytes())
    }

    fn join_many<S>(&self, channels: impl AsRef<[S]>, keys: Option<impl AsRef<[S]>>)
    where
        S: AsRef<str>,
    {
        let channels = join_with(channels.as_ref().iter().map(|s| s.as_ref()), ",");
        let msg = if keys.is_some() {
            let keys = join_with(keys.unwrap().as_ref().iter().map(|s| s.as_ref()), ",");
            format!("JOIN {} {}\r\n", channels, keys)
        } else {
            format!("JOIN {}\r\n", channels)
        };
        self.write(msg.as_bytes())
    }

    // this should actually take in a Channel not a &str..

    fn part(&self, channel: impl AsRef<str>, reason: impl AsRef<str>) {
        self.write(&format!("PART {} :{}\r\n", channel.as_ref(), reason.as_ref()).as_bytes());
    }

    fn part_many<S>(&self, channels: impl AsRef<[S]>, reason: impl AsRef<str>)
    where
        S: AsRef<str>,
    {
        let channels = join_with(channels.as_ref().iter().map(|s| s.as_ref()), ",");
        self.write(&format!("PART {} :{}\r\n", channels, reason.as_ref()).as_bytes());
    }

    fn write(&self, data: &[u8]);

    fn close(&self);
}

impl IrcClient for Client {
    fn write(&self, data: &[u8]) {
        let inner = &self.inner.read().unwrap();
        inner.write(data);
    }

    fn close(&self) {
        let inner = &self.inner.write().unwrap();
        inner.close();
    }
}

impl IrcClient for Inner {
    fn write(&self, data: &[u8]) {
        use std::io::Write;
        use std::str;

        // trim the \r\n
        trace!(
            ">> {}",
            str::from_utf8(&data[..data.len() - 2]).expect("valid utf-8")
        );

        // TODO split this as 510 chunks (512 - CLRF)
        self.write
            .lock()
            .unwrap()
            .write_all(data)
            .expect("IrcClient write");
    }

    fn close(&self) {
        use std::net::Shutdown;
        self.read
            .shutdown(Shutdown::Both)
            .expect("shutdown TcpStream");
    }
}

fn join_with<'a>(i: impl Iterator<Item = &'a str>, s: &str) -> String {
    let s = i.fold(String::new(), |mut a, c| {
        a.push_str(c);
        a.push_str(s);
        a
    });
    s.trim_right_matches(',').to_owned()
}
