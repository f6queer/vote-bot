use crate::bot::PollService;
use crate::constants::*;
use crate::poll_token::PollToken;
use sqlite::{Connection, OpenFlags};
use std::fs::File;
use std::path::Path;
use telegram_bot::types::{MessageId, UserId};

pub struct DbService {
    db: Connection,
}

impl DbService {
    pub fn new() -> Result<Self, sqlite::Error> {
        let mut is_exists = false;
        if !Path::new(DB_PATH).exists() {
            File::create(DB_PATH).ok();
        } else {
            is_exists = true;
        }
        let db = Connection::open_with_flags(DB_PATH, OpenFlags::new().set_read_write())?;
        let mut t = Self { db };
        if !is_exists {
            t.init().ok();
        }
        Ok(t)
    }

    pub fn init(&mut self) -> Result<(), sqlite::Error> {
        self.db.execute(
            "
        CREATE TABLE votes (token TEXT, user INTEGER, msg_id INTEGER);
        CREATE TABLE stats (id INTEGER, name TEXT, votes INTEGER);
        CREATE TABLE info (start INTEGER, end INTEGER, key TEXT);
        ",
        )
    }

    pub fn is_present(&mut self) -> Result<bool, sqlite::Error> {
        let mut exists = false;
        self.db.iterate("SELECT * FROM info;", |_| {
            exists = true;
            true
        })?;
        Ok(exists)
    }

    pub fn create(&mut self, poll: PollService) -> Result<(), sqlite::Error> {
        self.db.execute(&format!(
            "INSERT INTO info VALUES ({}, {}, '{}');",
            poll.start, poll.end, poll.key
        ))?;
        for i in 0..poll.candidates.len() {
            self.db.execute(&format!(
                "INSERT INTO stats VALUES ({}, '{}', 0);",
                i + 1,
                poll.candidates[i]
            ))?;
        }
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), sqlite::Error> {
        self.db.execute(
            "
        DELETE FROM info;
        DELETE FROM stats;
        DELETE FROM votes;
        ",
        )?;
        Ok(())
    }

    pub fn update(&mut self, idx: usize, val: i64) -> Result<(), sqlite::Error> {
        self.db.execute(&format!(
            "UPDATE stats SET votes = {} WHERE id = {};",
            val, idx
        ))?;
        Ok(())
    }

    pub fn load(&mut self) -> Result<PollService, sqlite::Error> {
        let mut poll = PollService::new(vec![], 0, 0);
        self.db.iterate("SELECT * FROM info", |pairs| {
            poll.start = pairs[0].1.unwrap().parse::<i64>().unwrap();
            poll.end = pairs[1].1.unwrap().parse::<i64>().unwrap();
            poll.key = pairs[2].1.unwrap().to_string();
            true
        })?;
        self.db.iterate("SELECT * FROM stats", |pairs| {
            poll.candidates.push(pairs[1].1.unwrap().to_string());
            poll.votes.push(pairs[2].1.unwrap().parse::<i64>().unwrap());
            true
        })?;
        Ok(poll)
    }

    pub fn fetch_token(&mut self, id: UserId) -> Result<Option<PollToken>, sqlite::Error> {
        let mut res = None;
        self.db
            .iterate(&format!("SELECT * FROM votes WHERE user={}", id), |pairs| {
                res = Some(PollToken {
                    token: pairs[0].1.unwrap().to_string(),
                    user_id: UserId::new(pairs[1].1.unwrap().parse::<i64>().unwrap()),
                    msg_id: MessageId::new(pairs[2].1.unwrap().parse::<i64>().unwrap()),
                });
                true
            })?;
        Ok(res)
    }

    pub fn remove_token(&mut self, id: UserId) -> Result<(), sqlite::Error> {
        self.db
            .execute(&format!("DELETE FROM votes WHERE user={};", id))?;
        Ok(())
    }

    pub fn insert_token(&mut self, token: PollToken) -> Result<(), sqlite::Error> {
        self.db.execute(&format!(
            "INSERT INTO votes VALUES ('{}', {}, {});",
            token.token, token.user_id, token.msg_id
        ))?;
        Ok(())
    }
}
