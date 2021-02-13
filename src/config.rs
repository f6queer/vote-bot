use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use telegram_bot::types::UserId;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Security {
    pub nonce: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub token: String,
    pub admins: Vec<(UserId, String)>,
    pub users: Vec<(UserId, String)>,
    pub group_chat: i64,
    pub security: Security,
}

impl Config {
    pub fn open(filename: &str) -> Result<Self, std::io::Error> {
        let mut f = File::open(filename)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(toml::from_str(&s).unwrap())
    }

    pub fn add_admin(&mut self, id: UserId, name: String) {
        self.admins.push((id, name));
    }

    pub fn add_user(&mut self, id: UserId, name: String) {
        self.users.push((id, name));
    }

    pub fn sync(&self, filename: &str) -> Result<(), std::io::Error> {
        let mut f = OpenOptions::new().write(true).open(filename)?;
        f.write_all(toml::to_string(self).unwrap().as_bytes())?;
        f.sync_all()?;
        Ok(())
    }
}
