use rand::prelude::*;
use sha3::{Digest, Sha3_256};
use std::collections::HashSet;

const TOKEN_ORIGIN_LENGTH: usize = 32;

pub struct TokenService {
    tokens: HashSet<String>,
}

impl TokenService {
    pub fn new() -> Self {
        Self {
            tokens: HashSet::new(),
        }
    }

    pub fn gen(&mut self) -> String {
        let mut token = self._gen();
        while let Err(_) = token {
            token = self._gen();
        }
        token.unwrap()
    }

    pub fn _gen(&mut self) -> Result<String, ()> {
        let mut rng = thread_rng();
        let origin: [u8; TOKEN_ORIGIN_LENGTH] = rng.gen();
        let mut hasher = Sha3_256::new();
        hasher.update(&origin);
        let result = hasher.finalize();
        let t = hex::encode(result);
        if !self.tokens.contains(&t) {
            self.tokens.insert(t.clone());
            Ok(t)
        } else {
            Err(())
        }
    }

    pub fn remove(&mut self, token: String) -> bool {
        if self.tokens.contains(&token) {
            self.tokens.remove(&token);
            true
        } else {
            false
        }
    }
}
