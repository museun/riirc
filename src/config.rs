use super::ui::*;

use std::collections::HashMap;
use std::io::Error as IoError;
use std::path::Path;
use std::{fmt, fs};
use toml_document::ParserError as TomlError;

#[derive(Debug, Default)]
pub struct Config {
    pub server: String,

    pub nick: String,
    pub user: String,
    pub real: String,
    pub pass: String,

    pub keybinds: Keybinds,
}

pub enum Error {
    CannotRead(IoError),
    CannotParse(TomlError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CannotRead(err) => {
                error!("CannotRead: {}", err);
                write!(f, "cannot read the config file")
            }
            Error::CannotParse(err) => {
                error!("CannotParse: {}", err);
                write!(f, "cannot parse the config file")
            }
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let data = fs::read_to_string(path.as_ref()).map_err(Error::CannotRead)?;

        use toml_document::*;
        let doc = Document::parse(&data).map_err(Error::CannotParse)?;

        let list = vec![
            "server".to_string(),
            "nick".to_string(),
            "user".to_string(),
            "real".to_string(),
            "pass".to_string(),
        ];

        let mut map: HashMap<String, Option<String>> = HashMap::new();
        for el in list {
            map.insert(el, None);
        }

        for child in doc.get_container(0).iter_children() {
            if let ValueRef::String(data) = child.value() {
                if let Some(val) = map.get_mut(child.key().get()) {
                    *val = Some(data.get().to_string());
                }
            }
        }

        let mut keybinds = Keybinds::default();
        for child in doc.get_container(1).iter_children() {
            if let ValueRef::String(data) = child.value() {
                if let Some(req) = KeyRequest::parse(child.key().get().to_string()) {
                    keybinds.insert(KeyType::from(data.get().to_string()), req)
                }
            }
        }

        Ok(Config {
            server: map.remove("server").unwrap().unwrap(),
            nick: map.remove("nick").unwrap().unwrap(),
            user: map.remove("user").unwrap().unwrap(),
            real: map.remove("real").unwrap().unwrap(),
            pass: map.remove("pass").unwrap().unwrap(),
            keybinds,
        })
    }

    pub fn dump(&self, w: &mut impl ::std::io::Write) {
        use toml_document::*;

        let mut doc = Document::new();
        let container = doc.insert_container(0, vec!["irc"].into_iter(), ContainerKind::Table);
        for (i, (k, v)) in [
            ("server", &self.server),
            ("nick", &self.nick),
            ("user", &self.user),
            ("real", &self.real),
            ("pass", &self.pass),
        ]
            .into_iter()
            .enumerate()
        {
            container.insert_string(i, k.to_string(), v.to_string());
        }

        let container = doc.insert_container(1, vec!["keybinds"].into_iter(), ContainerKind::Table);
        for (i, (v, k)) in self.keybinds.iter().enumerate() {
            let _s = container.insert_string(i, format!("{}", v), format!("{}", k));
        }

        writeln!(w, "{}", doc.to_string()).expect("to write config");
    }

    pub fn save(&self) {
        let mut file = fs::File::create("riirc.toml").expect("to create file");
        self.dump(&mut file);
    }
}
