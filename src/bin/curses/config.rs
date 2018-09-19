use super::*;

use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Config {
    pub server: String,

    pub nick: String,
    pub user: String,
    pub real: String,
    pub pass: String,

    pub keybinds: Keybinds,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ()> {
        let data = fs::read_to_string(path.as_ref()).map_err(|e| {
            error!("cannot read config file: {}", e);
            ()
        })?;

        use toml_document::*;

        let doc = Document::parse(&data).map_err(|e| {
            error!("cannot parse config file: {}", e);
            ()
        })?;

        use std::collections::HashMap;
        use std::convert::TryFrom;

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
                if let Ok(req) = KeyRequest::try_from(data.get().to_string()) {
                    keybinds.insert(KeyType::from(child.key().get().to_string()), req)
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
}

impl Drop for Config {
    fn drop(&mut self) {
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
            let s = container.insert_string(i, format!("{}", v), format!("{}", k));
        }

        fs::write("riirc.toml", doc.to_string()).expect("to write config");
    }
}
