use crate::constants::*;
use aes_gcm_siv::aead::{generic_array::GenericArray, Aead, NewAead};
use aes_gcm_siv::Aes256GcmSiv;
use rand::prelude::*;
use telegram_bot::types::{MessageId, UserId};

#[derive(Clone, Debug)]
pub struct PollToken {
    pub token: String,
    pub user_id: UserId,
    pub msg_id: MessageId,
}

impl PollToken {
    pub fn new(
        nonce_str: String,
        pub_key: String,
        priv_key: String,
        num: i64,
        user_id: UserId,
        msg_id: MessageId,
    ) -> PollToken {
        let mut rng = thread_rng();
        let mut key = vec![];
        key.append(&mut hex::decode(pub_key).unwrap());
        key.append(&mut hex::decode(priv_key).unwrap());
        let real_key = GenericArray::from_slice(&key);
        let cipher = Aes256GcmSiv::new(real_key);
        assert_eq!(nonce_str.len() <= NONCE_LEN, true);
        let mut nonce = nonce_str.as_bytes().to_vec();
        nonce.resize(NONCE_LEN, 0);
        let real_nonce = GenericArray::from_slice(&nonce);
        let ciphertext = cipher
            .encrypt(real_nonce, num.to_string().as_bytes())
            .expect("encryption failure!");

        Self {
            token: hex::encode(ciphertext),
            user_id,
            msg_id,
        }
    }

    pub fn decrypt(&self, nonce_str: String, pub_key: String, priv_key: String) -> Result<i64, ()> {
        if let Ok(priv_key_slice) = hex::decode(priv_key) {
            let mut key = vec![];
            key.append(&mut hex::decode(pub_key).unwrap());
            key.append(&mut priv_key_slice.to_vec());
            let real_key = GenericArray::from_slice(&key);
            let cipher = Aes256GcmSiv::new(real_key);
            assert_eq!(nonce_str.len() <= NONCE_LEN, true);
            let mut nonce = nonce_str.as_bytes().to_vec();
            nonce.resize(NONCE_LEN, 0);
            let real_nonce = GenericArray::from_slice(&nonce);
            if let Ok(plaintext) = cipher.decrypt(
                real_nonce,
                hex::decode(self.token.clone()).unwrap().as_ref(),
            ) {
                if let Ok(encoded) = String::from_utf8(plaintext) {
                    if let Ok(res) = encoded.parse::<i64>() {
                        Ok(res)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}
