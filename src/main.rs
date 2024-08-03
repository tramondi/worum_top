use teloxide::{
    prelude::*,
    types::ParseMode,
    utils::{
        markdown as md,
        command::BotCommands,
    },
};
use dotenv::dotenv;
use scraper::{Html, Selector};

#[derive(Debug,Default)]
struct WorumThread {
    pub title: String,
    pub link: String,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Поддерживаемые команды:")]
enum Command {
    #[command(description = "Топ тема на Woman-форуме")]
    Top,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting WorumTop bot");

    let bot = Bot::from_env();

    Command::repl(bot, command_top).await;
}

async fn forum_get_top_threads() -> Option<Vec<WorumThread>> {
    let mut threads = Vec::<WorumThread>::new();

    let resp = reqwest::get("https://woman.ru/forum/?sort=1d").await;
    if resp.is_err() {
        return None
    }

    let content = resp.unwrap().text().await;
    if content.is_err() {
        return None
    }

    let content = content.unwrap();

    let html = Html::parse_document(&content);

    let title_selector = Selector::parse(".list-item__title").unwrap();
    let link_selector = Selector::parse(".list-item__link").unwrap();
    let item_selector = Selector::parse(".list-item").unwrap();

    for node in html.select(&item_selector) {
        let mut thread = WorumThread::default();

        if let Some(title) = node.select(&title_selector).nth(0) {
            thread.title = title.inner_html();
        }

        if let Some(link) = node.select(&link_selector).nth(0) {
            thread.link = link.value().attr("href").unwrap().to_string();
        }

        threads.push(thread);
    }

    Some(threads)
}

async fn command_top(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Top => {
            let worum_top = forum_get_top_threads().await;
            if worum_top.is_none() {
                return Ok(())
            }

            let worum_top = worum_top.unwrap();
            if worum_top.len() == 0 {
                log::error!("zero top");
                return Ok(())
            }

            let thread = &worum_top[0];
            let title = &thread.title;
            let link = format!("https://woman.ru{}", thread.link);

            let answer = format!("Тема дня: {}", md::link(&link, title));

            bot.send_message(msg.chat.id, answer)
                .parse_mode(ParseMode::MarkdownV2)
                .send()
                .await?;
        }
    };

    Ok(())
}
