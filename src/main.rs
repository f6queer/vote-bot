mod bot;
mod config;
mod constants;
mod middlewares;
mod poll_token;
mod token_service;

use futures::StreamExt;



use telegram_bot::{
    Api, Error, Message, MessageKind, Poll, PollAnswer, SendPoll, UpdateKind, User,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = config::Config::open(constants::CONFIG_PATH).unwrap();

    let api = Api::new(config.token);
    let mut stream = api.stream();
    let mut bot = bot::Bot::new();

    while let Some(update) = stream.next().await {
        let update = update?;

        match update.kind {
            UpdateKind::Message(message) => match message.kind {
                MessageKind::Text { ref data, .. } => match data.split_whitespace().nth(0).unwrap()
                {
                    "/create" => {
                        bot.handle_create_poll(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    "/remove" => bot.handle_remove_poll(api.clone(), message.clone()).await?,
                    "/poll" => bot.handle_poll(api.clone(), message.clone()).await?,
                    "/help" => bot.handle_help(api.clone(), message.clone()).await?,
                    "/admin_help" => bot.handle_admin_help(api.clone(), message.clone()).await?,
                    "/vote" => {
                        bot.handle_vote(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    "/undo" => {
                        bot.handle_undo(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    "/add_user" => bot.handle_add_user(api.clone(), message.clone()).await?,
                    "/accept" => {
                        bot.handle_accept(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    "/add_admin" => bot.handle_add_admin(api.clone(), message.clone()).await?,
                    "/accept_admin" => {
                        bot.handle_accept_admin(api.clone(), message.clone(), data.to_string())
                            .await?
                    }
                    _ => {
                        bot.handle_unknown_command(api.clone(), message.clone())
                            .await?
                    }
                },
                _ => (),
            },
            _ => (),
        }
    }

    Ok(())
}
