use crate::config::Config;
use crate::constants::*;
use crate::middlewares::db::DbService;
use crate::poll_token::PollToken;
use crate::token_service::TokenService;
use chrono::prelude::*;
use chrono_tz::Asia::Seoul;
use rand::prelude::*;
use std::collections::{HashSet};

use telegram_bot::prelude::*;

use telegram_bot::types::{MessageChat, ParseMode, UserId};

use telegram_bot::{
    Api, Error, Message,
};

#[derive(Clone)]
pub struct PollService {
    pub candidates: Vec<String>,
    pub start: i64,
    pub end: i64,
    pub key: String,
    pub votes: Vec<i64>,
}

impl PollService {
    pub fn new(candidates: Vec<String>, start: i64, end: i64) -> Self {
        let mut rng = thread_rng();
        let key: [u8; AES_KEY_LEN / 2] = rng.gen();
        Self {
            candidates: candidates.clone(),
            start,
            end,
            key: hex::encode(key),
            votes: vec![0; candidates.len()],
        }
    }
}

pub struct Bot {
    is_present: bool,
    db: DbService,
    poll: PollService,
    user_token: TokenService,
    admin_token: TokenService,
    config: Config,
    admins: HashSet<UserId>,
    users: HashSet<UserId>,
}

impl Bot {
    pub fn new() -> Self {
        let config = Config::open(CONFIG_PATH).unwrap();
        let mut admins: HashSet<UserId> = HashSet::new();
        let mut users: HashSet<UserId> = HashSet::new();
        for (u, _) in config.users.clone() {
            users.insert(u);
        }
        for (u, _) in config.admins.clone() {
            admins.insert(u);
        }
        let mut db = DbService::new().unwrap();
        let poll = db.load().unwrap();
        Self {
            is_present: db.is_present().unwrap(),
            db,
            poll,
            user_token: TokenService::new(),
            admin_token: TokenService::new(),
            config,
            admins,
            users,
        }
    }

    // ADMIN ONLY
    pub async fn handle_remove_poll<'p>(
        &mut self,
        api: Api,
        message: Message,
    ) -> Result<(), Error> {
        if self.admins.contains(&message.from.id) {
            if self.is_present {
                self.is_present = false;
                api.send(message.text_reply("투표가 종료되었습니다."))
                    .await?;
                let mut result = format!("**결과 안내**\n");
                let mut res = vec![];
                for i in 0..self.poll.candidates.len() {
                    res.push((self.poll.votes[i], i));
                }
                res.sort();
                res.reverse();
                use std::cmp::min;
                for i in 0..min(3, self.poll.candidates.len()) {
                    result.push_str(&format!(
                        "{}위: 기호 {}번 후보자 {} ({}표)\n",
                        i + 1,
                        res[i].1 + 1,
                        self.poll.candidates[res[i].1],
                        res[i].0
                    ));
                }
                result.push_str("당선을 축하드립니다!");
                self.db.clear().ok();
                api.send(message.text_reply(&result).parse_mode(ParseMode::Markdown))
                    .await?;
            } else {
                api.send(message.text_reply("죄송합니다. 아직 투표가 열려있지 않은 것 같습니다."))
                    .await?;
            }
        } else {
            api.send(
                message
                    .text_reply("죄송합니다. 관리자 전용 명령어 입니다.")
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        }
        Ok(())
    }

    // ADMIN ONLY
    pub async fn handle_create_poll(
        &mut self,
        api: Api,
        message: Message,
        command: String,
    ) -> Result<(), Error> {
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if let Some(parms) = splited.get(1..) {
            if parms.len() > 0 {
                if self.admins.contains(&message.from.id) {
                    let mut candidates = parms.to_vec();
                    if let Ok(times) = candidates.pop().unwrap().parse::<i64>() {
                        if candidates.len() == 0 {
                            api.send(
                                message
                                    .text_reply("죄송합니다. 후보자는 한명 이상 입력하셔야 합니다.")
                                    .parse_mode(ParseMode::Markdown),
                            )
                            .await?;
                        } else {
                            if times <= 0 {
                                api.send(
                                    message
                                        .text_reply(
                                            "죄송합니다. 투표는 `1분` 이상 진행되어야 합니다.",
                                        )
                                        .parse_mode(ParseMode::Markdown),
                                )
                                .await?;
                            } else {
                                if self.is_present {
                                    api.send(
                                        message
                                            .text_reply(
                                                "죄송합니다. 투표가 이미 진행 중인 것 같습니다.",
                                            )
                                            .parse_mode(ParseMode::Markdown),
                                    )
                                    .await?;
                                } else {
                                    self.is_present = true;
                                    self.poll = PollService::new(
                                        candidates,
                                        Utc::now().timestamp(),
                                        Utc::now().timestamp() + times * 60,
                                    );
                                    self.db.create(self.poll.clone()).ok();
                                    self.handle_poll(api.clone(), message.clone()).await?;
                                }
                            }
                        }
                    } else {
                        self.handle_unknown_command(api.clone(), message.clone())
                            .await?;
                    }
                } else {
                    api.send(
                        message
                            .text_reply("죄송합니다. 관리자 전용 명령어 입니다.")
                            .parse_mode(ParseMode::Markdown),
                    )
                    .await?;
                }
            } else {
                self.handle_unknown_command(api.clone(), message.clone())
                    .await?;
            }
        } else {
            self.handle_unknown_command(api.clone(), message.clone())
                .await?;
        }
        Ok(())
    }

    pub async fn handle_poll(&self, api: Api, message: Message) -> Result<(), Error> {
        if self.is_present {
            let mut reply_msg = format!(
                "**{}**\n총 후보 수: {}\n",
                TITLE_NAME,
                self.poll.candidates.len()
            );
            for i in 0..self.poll.candidates.len() {
                reply_msg.push_str(&format!(
                    "기호 **{}**번: {}\n",
                    i + 1,
                    self.poll.candidates[i]
                ));
            }
            reply_msg.push_str("투표 방법: `/vote [후보자 번호]`\n");
            reply_msg.push_str(&format!(
                "시작: {}\n",
                Seoul
                    .from_local_datetime(&NaiveDateTime::from_timestamp(self.poll.start, 0))
                    .unwrap()
            ));
            reply_msg.push_str(&format!(
                "종료: {}\n",
                Seoul
                    .from_local_datetime(&NaiveDateTime::from_timestamp(self.poll.end, 0))
                    .unwrap()
            ));
            api.send(
                message
                    .text_reply(&reply_msg)
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        } else {
            api.send(message.text_reply("죄송합니다. 아직 투표가 열려있지 않은 것 같습니다."))
                .await?;
        }
        Ok(())
    }

    pub async fn handle_unknown_command(&self, api: Api, message: Message) -> Result<(), Error> {
        api.send(message.text_reply("죄송합니다. 알 수 없는 명령어가 입력되었습니다.\n도움말을 보시려면 `/help` 명령어를 입력해주세요.").parse_mode(ParseMode::Markdown)).await?;
        Ok(())
    }

    pub async fn handle_help(&self, api: Api, message: Message) -> Result<(), Error> {
        api.send(message.text_reply(HELP).parse_mode(ParseMode::Markdown))
            .await?;
        Ok(())
    }

    pub async fn handle_admin_help(&self, api: Api, message: Message) -> Result<(), Error> {
        api.send(
            message
                .text_reply(ADMIN_HELP)
                .parse_mode(ParseMode::Markdown),
        )
        .await?;
        Ok(())
    }

    pub async fn handle_vote(
        &mut self,
        api: Api,
        message: Message,
        command: String,
    ) -> Result<(), Error> {
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if let Some(parms) = splited.get(1..) {
            if parms.len() == 0 || parms.len() > 1 {
                self.handle_unknown_command(api.clone(), message.clone())
                    .await?;
            } else {
                if let Ok(target) = parms[0].parse::<i64>() {
                    if self.is_present {
                        if target <= 0 || target > self.poll.candidates.len() as i64 {
                            api.send(
                                message
                                    .text_reply("죄송합니다. 올바른 후보 번호를 입력해주세요.")
                                    .parse_mode(ParseMode::Markdown),
                            )
                            .await?;
                        } else {
                            if let Some(t) = self.db.fetch_token(message.from.id).unwrap() {
                                use telegram_bot::types::requests::SendMessage;
                                let mut msg = SendMessage::new(message.chat, "이미 투표가 되어있습니다. 위 메시지 이후에 투표를 취소하기 위한 명령어 `/undo ~` 를 복사하여 입력해주십시오.");
                                api.send(msg.reply_to(t.msg_id).parse_mode(ParseMode::Markdown))
                                    .await?;
                            } else {
                                self.poll.votes[target as usize - 1] += 1;
                                self.db
                                    .update(target as usize, self.poll.votes[target as usize - 1])
                                    .ok();
                                let (dec, poll_token) = PollToken::new(
                                    self.config.security.nonce.clone(),
                                    self.poll.key.clone(),
                                    target,
                                    message.from.id,
                                    message.id,
                                );
                                self.db.insert_token(poll_token).ok();
                                api.send(message.text_reply(&format!("투표해주셔서 감사합니다. {} 후보에게 정상적으로 투표가 완료되었습니다.\n투표를 취소하려면 `/undo {}` 커맨드를 입력해주세요.", self.poll.candidates[target as usize - 1], dec)).parse_mode(ParseMode::Markdown)).await?;
                            }
                        }
                    } else {
                        api.send(
                            message
                                .text_reply("죄송합니다. 현재 투표가 진행중이 아닙니다.")
                                .parse_mode(ParseMode::Markdown),
                        )
                        .await?;
                    }
                } else {
                    self.handle_unknown_command(api.clone(), message.clone())
                        .await?;
                }
            }
        } else {
            self.handle_unknown_command(api.clone(), message.clone())
                .await?;
        }
        Ok(())
    }

    pub async fn handle_undo(
        &mut self,
        api: Api,
        message: Message,
        command: String,
    ) -> Result<(), Error> {
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if let Some(parms) = splited.get(1..) {
            if parms.len() == 0 || parms.len() > 1 {
                self.handle_unknown_command(api.clone(), message.clone())
                    .await?;
            } else {
                if let Some(t) = self.db.fetch_token(message.from.id).unwrap() {
                    if let Ok(vote) = t.decrypt(
                        self.config.security.nonce.clone(),
                        self.poll.key.clone(),
                        parms[0].clone(),
                    ) {
                        self.poll.votes[vote as usize - 1] -= 1;
                        self.db
                            .update(vote as usize, self.poll.votes[vote as usize - 1])
                            .ok();
                        self.db.remove_token(message.from.id).ok();
                        api.send(
                            message
                                .text_reply("정상적으로 취소가 되었습니다.")
                                .parse_mode(ParseMode::Markdown),
                        )
                        .await?;
                    } else {
                        api.send(
                            message
                                .text_reply("죄송합니다. 토큰이 올바르지 않습니다.")
                                .parse_mode(ParseMode::Markdown),
                        )
                        .await?;
                    }
                } else {
                    api.send(
                        message
                            .text_reply("죄송합니다. 투표가 되어있지 않습니다.")
                            .parse_mode(ParseMode::Markdown),
                    )
                    .await?;
                }
            }
        } else {
            self.handle_unknown_command(api.clone(), message.clone())
                .await?;
        }
        Ok(())
    }

    // ADMIN ONLY
    pub async fn handle_add_user(&mut self, api: Api, message: Message) -> Result<(), Error> {
        if let MessageChat::Private(_) = message.chat {
            if self.admins.contains(&message.from.id) {
                api.send(
                    message
                        .text_reply(&format!(
                            "토큰이 생성되었습니다. 꼭 대상자에게만 지급하십시오. 토큰: `{}`",
                            self.user_token.gen()
                        ))
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            } else {
                api.send(
                    message
                        .text_reply("죄송합니다. 관리자 전용 명령어 입니다.")
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            }
        } else {
            api.send(
                message
                    .text_reply(
                        "죄송합니다. 관리자 전용 명령어는 개인 챗에서만 이용할 수 있습니다.",
                    )
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        }
        Ok(())
    }

    pub async fn handle_accept(
        &mut self,
        api: Api,
        message: Message,
        command: String,
    ) -> Result<(), Error> {
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if let Some(parms) = splited.get(1..) {
            if self.users.contains(&message.from.id) {
                api.send(
                    message
                        .text_reply("죄송합니다. 이미 등록되어있습니다.")
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            } else {
                let token = parms[0].clone();
                if self.user_token.remove(token) {
                    self.users.insert(message.from.id);
                    self.config
                        .add_user(message.clone().from.id, message.clone().from.first_name);
                    self.config.sync(CONFIG_PATH).ok();
                    api.send(
                        message
                            .text_reply("정상적으로 등록이 완료되었습니다.")
                            .parse_mode(ParseMode::Markdown),
                    )
                    .await?;
                } else {
                    api.send(
                        message
                            .text_reply("죄송합니다. 입력하신 토큰이 유효하지 않습니다.")
                            .parse_mode(ParseMode::Markdown),
                    )
                    .await?;
                }
            }
        } else {
            self.handle_unknown_command(api.clone(), message.clone())
                .await?;
        }
        Ok(())
    }

    // ADMIN ONLY
    pub async fn handle_add_admin(&mut self, api: Api, message: Message) -> Result<(), Error> {
        if let MessageChat::Private(_) = message.chat {
            if self.admins.contains(&message.from.id) {
                api.send(
                    message
                        .text_reply(&format!(
                            "토큰이 생성되었습니다. 꼭 대상자에게만 지급하십시오. 토큰: `{}`",
                            self.admin_token.gen()
                        ))
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            } else {
                api.send(
                    message
                        .text_reply("죄송합니다. 관리자 전용 명령어 입니다.")
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            }
        } else {
            api.send(
                message
                    .text_reply(
                        "죄송합니다. 관리자 전용 명령어는 개인 챗에서만 이용할 수 있습니다.",
                    )
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        }
        Ok(())
    }

    pub async fn handle_accept_admin(
        &mut self,
        api: Api,
        message: Message,
        command: String,
    ) -> Result<(), Error> {
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if let Some(parms) = splited.get(1..) {
            if self.admins.contains(&message.from.id) {
                api.send(
                    message
                        .text_reply("죄송합니다. 이미 등록되어있습니다.")
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            } else {
                let token = parms[0].clone();
                if self.admin_token.remove(token) {
                    self.admins.insert(message.from.id);
                    self.config
                        .add_admin(message.clone().from.id, message.clone().from.first_name);
                    self.config.sync(CONFIG_PATH).ok();
                    api.send(
                        message
                            .text_reply("정상적으로 등록이 완료되었습니다.")
                            .parse_mode(ParseMode::Markdown),
                    )
                    .await?;
                } else {
                    api.send(
                        message
                            .text_reply("죄송합니다. 입력하신 토큰이 유효하지 않습니다.")
                            .parse_mode(ParseMode::Markdown),
                    )
                    .await?;
                }
            }
        } else {
            self.handle_unknown_command(api.clone(), message.clone())
                .await?;
        }
        Ok(())
    }
}
