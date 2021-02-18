use crate::config::Config;
use crate::constants::*;
use crate::middlewares::db::DbService;
use crate::poll_token::PollToken;
use crate::token_service::TokenService;
use chrono::prelude::*;
use chrono_tz::Asia::Seoul;
use rand::prelude::*;
use std::collections::HashSet;

use telegram_bot::prelude::*;

use telegram_bot::types::reply_markup::*;
use telegram_bot::types::{CallbackQuery, MessageChat, MessageId, ParseMode, UserId};

use telegram_bot::{Api, Error, Message};

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
    //user_token: TokenService,
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
            //user_token: TokenService::new(),
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
                let mut result = format!("*결과 안내*\n");
                let mut res = vec![];
                for i in 0..self.poll.candidates.len() {
                    res.push((self.poll.votes[i], i));
                }
                res.sort();
                res.reverse();
                //use std::cmp::min;
                for i in 0..self.poll.candidates.len() {
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
                "{}\n총 후보 수: {}\n",
                TITLE_NAME,
                self.poll.candidates.len()
            );
            let mut markup = InlineKeyboardMarkup::new();
            for i in 0..self.poll.candidates.len() {
                reply_msg.push_str(&format!(
                    "*기호 {}번*: {}\n",
                    i + 1,
                    self.poll.candidates[i]
                ));
            }
            reply_msg.push_str("투표 방법: 버튼을 클릭하세요.\n");
            reply_msg.push_str(&format!(
                "*시작*: {}\n",
                Seoul.from_utc_datetime(&NaiveDateTime::from_timestamp(self.poll.start, 0)) //.unwrap()
            ));
            reply_msg.push_str(&format!(
                "*종료*: {}\n",
                Seoul.from_utc_datetime(&NaiveDateTime::from_timestamp(self.poll.end, 0)) //.unwrap()
            ));
            markup.add_row(vec![InlineKeyboardButton::url(
                "투표하러 가기",
                "https://t.me/F6PollBot",
            )]);
            api.send(
                message
                    .text_reply(&reply_msg)
                    .reply_markup(ReplyMarkup::InlineKeyboardMarkup(markup))
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        } else {
            api.send(message.text_reply("죄송합니다. 아직 투표가 열려있지 않은 것 같습니다."))
                .await?;
        }
        Ok(())
    }

    pub async fn handle_vote(&self, api: Api, message: Message) -> Result<(), Error> {
        if let MessageChat::Private(_) = message.chat {
            if self.is_present {
                let mut reply_msg = format!(
                    "{}\n총 후보 수: {}\n",
                    TITLE_NAME,
                    self.poll.candidates.len()
                );
                let mut markup = InlineKeyboardMarkup::new();
                let mut rng = thread_rng();
                let priv_key: [u8; AES_KEY_LEN / 2] = rng.gen();
                for i in 0..self.poll.candidates.len() {
                    reply_msg.push_str(&format!(
                        "*기호 {}번*: {}\n",
                        i + 1,
                        self.poll.candidates[i]
                    ));
                    markup.add_row(vec![InlineKeyboardButton::callback(
                        &format!("기호 {}번", i + 1),
                        &format!("/vote {} {}", i + 1, hex::encode(priv_key)),
                    )]);
                }
                markup.add_row(vec![InlineKeyboardButton::callback(
                    "투표한 후보 보기",
                    &format!("/check {}", hex::encode(priv_key)),
                )]);
                markup.add_row(vec![InlineKeyboardButton::callback(
                    "다시 투표하기",
                    &format!("/clear {}", hex::encode(priv_key)),
                )]);
                reply_msg.push_str("투표 방법: 버튼을 클릭하세요.\n");
                reply_msg.push_str(&format!(
                    "*시작*: {}\n",
                    Seoul.from_utc_datetime(&NaiveDateTime::from_timestamp(self.poll.start, 0)) //.unwrap()
                ));
                reply_msg.push_str(&format!(
                    "*종료*: {}\n",
                    Seoul.from_utc_datetime(&NaiveDateTime::from_timestamp(self.poll.end, 0)) //.unwrap()
                ));
                api.send(
                    message
                        .text_reply(&reply_msg)
                        .reply_markup(ReplyMarkup::InlineKeyboardMarkup(markup))
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            } else {
                api.send(message.text_reply("죄송합니다. 아직 투표가 열려있지 않은 것 같습니다."))
                    .await?;
            }
        } else {
            api.send(message.text_reply("이 명령어는 개인 대화에서만 사용하실 수 있습니다."))
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

    pub async fn handle_check_callback(
        &mut self,
        api: Api,
        callback: CallbackQuery,
    ) -> Result<(), Error> {
        let command = callback.data.clone().unwrap();
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if self.is_present {
            if self.users.contains(&callback.from.id) {
                let mut cnt = 0;
                let list = self.db.fetch_token(callback.from.id).unwrap();
                let mut res = String::new();
                for t in list.clone() {
                    if let Ok(vote) = t.decrypt(
                        self.config.security.nonce.clone(),
                        self.poll.key.clone(),
                        splited[1].clone(),
                    ) {
                        res.push_str(&format!(
                            "{}번 후보: {} ",
                            cnt + 1,
                            self.poll.candidates[vote as usize - 1]
                        ));
                        cnt += 1;
                    }
                }
                if res.is_empty() {
                    api.send(callback.answer("현재 투표한 후보가 없습니다."))
                        .await?;
                } else {
                    api.send(callback.answer(&res)).await?;
                }
            } else {
                api.send(callback.answer("죄송합니다. 투표는 허용된 유저만 할 수 있습니다."))
                    .await?;
            }
        } else {
            api.send(callback.answer("죄송합니다. 현재 투표가 진행중이 아닙니다."))
                .await?;
        }
        Ok(())
    }

    pub async fn handle_clear_callback(
        &mut self,
        api: Api,
        callback: CallbackQuery,
    ) -> Result<(), Error> {
        let command = callback.data.clone().unwrap();
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if self.is_present {
            if self.users.contains(&callback.from.id) {
                let list = self.db.fetch_token(callback.from.id).unwrap();
                for t in list.clone() {
                    if let Ok(vote) = t.decrypt(
                        self.config.security.nonce.clone(),
                        self.poll.key.clone(),
                        splited[1].clone(),
                    ) {
                        self.poll.votes[vote as usize - 1] -= 1;
                        self.db
                            .update(vote as usize, self.poll.votes[vote as usize - 1])
                            .ok();
                        self.db.remove_token(callback.from.id, t.token).ok();
                    }
                }
                api.send(callback.answer("투표가 성공적으로 초기화 되었습니다."))
                    .await?;
            } else {
                api.send(callback.answer("죄송합니다. 투표는 허용된 유저만 할 수 있습니다."))
                    .await?;
            }
        } else {
            api.send(callback.answer("죄송합니다. 현재 투표가 진행중이 아닙니다."))
                .await?;
        }
        Ok(())
    }

    pub async fn handle_vote_callback(
        &mut self,
        api: Api,
        callback: CallbackQuery,
    ) -> Result<(), Error> {
        let command = callback.data.clone().unwrap();
        let splited: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if let Ok(target) = splited[1].parse::<i64>() {
            if self.is_present {
                if self.users.contains(&callback.from.id) {
                    let mut cnt = 0;
                    let list = self.db.fetch_token(callback.from.id).unwrap();
                    for t in list.clone() {
                        if let Ok(_) = t.decrypt(
                            self.config.security.nonce.clone(),
                            self.poll.key.clone(),
                            splited[2].clone(),
                        ) {
                            cnt += 1;
                        }
                    }
                    if cnt == list.len() {
                        let mut r = 0;
                        for t in list.clone() {
                            if let Ok(vote) = t.decrypt(
                                self.config.security.nonce.clone(),
                                self.poll.key.clone(),
                                splited[2].clone(),
                            ) {
                                if vote == target {
                                    self.poll.votes[vote as usize - 1] -= 1;
                                    self.db
                                        .update(vote as usize, self.poll.votes[vote as usize - 1])
                                        .ok();
                                    self.db.remove_token(callback.from.id, t.token).ok();
                                    r += 1;
                                    break;
                                }
                            }
                        }
                        if r != 0 {
                            api.send(callback.answer("정상적으로 투표가 취소되었습니다."))
                                .await?;
                        } else {
                            if cnt == 3 {
                                for t in list.clone() {
                                    if let Ok(vote) = t.decrypt(
                                        self.config.security.nonce.clone(),
                                        self.poll.key.clone(),
                                        splited[2].clone(),
                                    ) {
                                        self.poll.votes[vote as usize - 1] -= 1;
                                        self.db
                                            .update(
                                                vote as usize,
                                                self.poll.votes[vote as usize - 1],
                                            )
                                            .ok();
                                        self.db.remove_token(callback.from.id, t.token).ok();
                                        break;
                                    }
                                }
                            }
                            self.poll.votes[target as usize - 1] += 1;
                            self.db
                                .update(target as usize, self.poll.votes[target as usize - 1])
                                .ok();
                            let poll_token = PollToken::new(
                                self.config.security.nonce.clone(),
                                self.poll.key.clone(),
                                splited[2].clone(),
                                target,
                                callback.from.id,
                                MessageId::new(0),
                            );
                            self.db.insert_token(poll_token).ok();
                            api.send(callback.answer(&format!("투표해주셔서 감사합니다. {} 후보에게 정상적으로 투표가 완료되었습니다.", self.poll.candidates[target as usize - 1]))).await?;
                        }
                    } else {
                        api.send(
                            callback.answer("죄송합니다. 투표한 메시지에서 다시 부탁드립니다."),
                        )
                        .await?;
                    }
                } else {
                    api.send(callback.answer("죄송합니다. 투표는 허용된 유저만 할 수 있습니다."))
                        .await?;
                }
            } else {
                api.send(callback.answer("죄송합니다. 현재 투표가 진행중이 아닙니다."))
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn handle_accept(&mut self, api: Api, message: Message) -> Result<(), Error> {
        if self.users.contains(&message.from.id) {
            api.send(
                message
                    .text_reply("죄송합니다. 이미 등록되어있습니다.")
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        } else {
            if message.chat.id() == self.config.group_chat {
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
                        .text_reply("죄송합니다. 권한이 없습니다.")
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
            }
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
                            "토큰이 생성되었습니다. 꼭 대상자에게만 지급하십시오. `/accept_admin {}`",
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
            if !(parms.len() == 0 || parms.len() > 1) {
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
        } else {
            self.handle_unknown_command(api.clone(), message.clone())
                .await?;
        }
        Ok(())
    }

    pub async fn handle_start(&mut self, api: Api, message: Message) -> Result<(), Error> {
        if let MessageChat::Private(_) = message.chat {
            api.send(
                message
                    .text_reply("F⁶ 임원 선거 봇에 오신걸 환영합니다!")
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
            if self.is_present {
                api.send(
                    message
                        .text_reply("현재 투표가 진행중입니다.")
                        .parse_mode(ParseMode::Markdown),
                )
                .await?;
                self.handle_vote(api.clone(), message.clone()).await?;
            }
        } else {
            api.send(
                message
                    .text_reply("이 명령어는 개인 대화에서만 쓸 수 있습니다.")
                    .parse_mode(ParseMode::Markdown),
            )
            .await?;
        }
        Ok(())
    }

    // ADMIN ONLY
    pub async fn handle_register_chat(&mut self, api: Api, message: Message) -> Result<(), Error> {
        if self.admins.contains(&message.from.id) {
            self.config.set_group_chat(message.chat.id());
            self.config.sync(CONFIG_PATH).ok();
            api.send(
                message
                    .text_reply("정상적으로 투표 관리 챗이 등록되었습니다.")
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
        Ok(())
    }
}
