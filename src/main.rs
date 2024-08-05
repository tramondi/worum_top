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
    #[command(description = "Тема дня")]
    Top,
    #[command(description = "Тема недели")]
    Week,
    #[command(description = "Тема месяца")]
    Month,
    #[command(description = "Топ тема за всё время")]
    Ever,
}

static WORUM_TOP_THREADS_DAY: &str = "https://woman.ru/forum/?sort=1d";
static WORUM_TOP_THREADS_WEEK: &str = "https://woman.ru/forum/?sort=7d";
static WORUM_TOP_THREADS_MONTH: &str = "https://woman.ru/forum/?sort=30d";
static WORUM_TOP_THREADS_EVER: &str = "https://woman.ru/forum/?sort=all";

static SELECTOR_THREAD: &str = ".list-item";
static SELECTOR_THREAD_TITLE: &str = ".list-item__title";
static SELECTOR_THREAD_LINK: &str = ".list-item__link";
static SELECTOR_THREAD_TEXT: &str = ".card_topic-start .card__comment";
static THREAD_TEXT_LIMIT: usize = 140;

#[tokio::main]
async fn main() {
    dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting WorumTop bot");

    let bot = Bot::from_env();

    Command::repl(bot, command_handle).await;
}

async fn fetch_content(url: &str) -> Option<String> {
    let Ok(resp) = reqwest::get(url).await else {
        return None
    };

    let Ok(content) = resp.text().await else {
        return None
    };

    Some(content)
}

async fn forum_get_threads(url: &str) -> Option<Vec<WorumThread>> {
    let mut threads = Vec::<WorumThread>::new();

    let Some(content) = fetch_content(url).await else {
        return None
    };

    let html = Html::parse_document(&content);

    let item_selector = Selector::parse(SELECTOR_THREAD).unwrap();
    let title_selector = Selector::parse(SELECTOR_THREAD_TITLE).unwrap();
    let link_selector = Selector::parse(SELECTOR_THREAD_LINK).unwrap();

    for node in html.select(&item_selector) {
        let mut thread = WorumThread::default();

        if let Some(title) = node.select(&title_selector).nth(0) {
            thread.title = title.inner_html();
        }

        if let Some(link) = node.select(&link_selector).nth(0) {
            let thread_path = link.value().attr("href").unwrap().to_string();
            thread.link = format!("https://woman.ru{}", thread_path);
        }

        threads.push(thread);
    }

    Some(threads)
}

async fn forum_get_thread_text(thread_url: &str) -> Option<String> {
    let mut text = String::default();

    let Some(content) = fetch_content(thread_url).await else {
        return None
    };

    let html = Html::parse_document(&content);

    let text_selector = Selector::parse(SELECTOR_THREAD_TEXT).unwrap();

    for node in html.select(&text_selector) {
        text = node.text().collect::<String>();
    }

    Some(text)
}

async fn command_handle(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let threads_url = match cmd {
        Command::Top => WORUM_TOP_THREADS_DAY,
        Command::Week => WORUM_TOP_THREADS_WEEK,
        Command::Month => WORUM_TOP_THREADS_MONTH,
        Command::Ever => WORUM_TOP_THREADS_EVER,
    };

    let Some(threads) = forum_get_threads(threads_url).await else {
        return Ok(())
    };

    if threads.len() == 0 {
        log::error!("no threads");
        return Ok(())
    }

    let thread = &threads[0];
    let title = &thread.title;
    let link = &thread.link;

    let Some(text) = forum_get_thread_text(link).await else {
        return Ok(())
    };

    let topic = md::link(link, title);
    let mut text = md::escape(&text);

    if text.len() > THREAD_TEXT_LIMIT {
        text = text.chars()
            .take(THREAD_TEXT_LIMIT)
            .collect::<String>();
        text += "…";
    }

    let text = md::italic(&text);
    let answer = format!("{}\n\n{}", topic, text);

    bot.send_message(msg.chat.id, answer)
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(())
}
