#[macro_use]
extern crate pkg_version;

mod bot;
mod config;
mod constants;
mod middlewares;
mod poll_token;
mod token_service;

use futures::StreamExt;
use tokio::time::{timeout, Duration};

use telegram_bot::{
    Api, Error, Message, MessageKind, Poll, PollAnswer, SendPoll, UpdateKind, User,
};

use chrono::prelude::*;

async fn check_poll(bot: &mut bot::Bot, api: Api) {
    if bot.is_present {
        //dbg!(bot.poll.end);
        if bot.poll.end <= Utc::now().timestamp() {
            bot.remove_poll(api).await.ok();
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = config::Config::open(constants::CONFIG_PATH).unwrap();

    let api = Api::new(config.token);
    let mut stream = api.stream();
    let mut bot = bot::Bot::new();
    //timeout(Duration::from_secs(1), check_poll(&mut bot, api.clone())).await.ok();

    while let Some(update) = stream.next().await {
        let update = update?;

        match update.kind {
            UpdateKind::Message(message) => match message.kind {
                MessageKind::Text { ref data, .. } => match data.split_whitespace().nth(0).unwrap().split('@').nth(0).unwrap()
                {
                    "/start" => bot.handle_start(api.clone(), message.clone()).await?,
                    "/register_chat" => {
                        bot.handle_register_chat(api.clone(), message.clone())
                            .await?
                    }
                    "/create" => {
                        bot.handle_create_poll(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    "/remove" => bot.handle_remove_poll(api.clone(), message.clone()).await?,
                    "/poll" => bot.handle_poll(api.clone(), message.clone()).await?,
                    "/vote" => bot.handle_vote(api.clone(), message.clone()).await?,
                    "/help" => bot.handle_help(api.clone(), message.clone()).await?,
                    "/about" => bot.handle_about(api.clone(), message.clone()).await?,
                    "/admin_help" => bot.handle_admin_help(api.clone(), message.clone()).await?,
                    /*"/vote" => {
                        bot.handle_vote(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    "/undo" => {
                        bot.handle_undo(api.clone(), message.clone(), data.to_string())
                            .await?
                    }*/
                    //"/add_user" => bot.handle_add_user(api.clone(), message.clone()).await?,
                    "/accept" => bot.handle_accept(api.clone(), message.clone()).await?,
                    "/add_admin" => bot.handle_add_admin(api.clone(), message.clone()).await?,
                    "/accept_admin" => {
                        bot.handle_accept_admin(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    _ => {}
                },
                _ => (),
            },
            UpdateKind::CallbackQuery(callback) => match callback
                .data
                .clone()
                .unwrap()
                .split_whitespace()
                .nth(0)
                .unwrap()
            {
                "/vote" => {
                    bot.handle_vote_callback(api.clone(), callback.clone())
                        .await?
                }
                "/check" => {
                    bot.handle_check_callback(api.clone(), callback.clone())
                        .await?
                }
                "/clear" => {
                    bot.handle_clear_callback(api.clone(), callback.clone())
                        .await?
                }
                _ => (),
            },
            _ => (),
        }
    }

    Ok(())
}
