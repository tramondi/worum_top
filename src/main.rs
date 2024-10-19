use teloxide::{
    prelude::*,
    types::{ParseMode, Me},
    utils::{
        html,
        command::BotCommands,
    },
};
use dotenv::dotenv;
use scraper::{Html, Selector};
use rand::prelude::*;
use std::error::Error;

#[derive(Debug,Default)]
struct WorumThread {
    pub title: String,
    pub link: String,
    pub photo_url: Option<String>,
}

#[derive(Debug,Default)]
struct SubrubricThread {
    pub title: String,
    pub section: String,
    pub subrubric: String,
    pub link: String,
    pub subrubric_link: String,
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

    #[command(description = "Случайная тема из рубрик")]
    Rubric,
}

static WORUM_TOP_THREADS_DAY: &str = "https://woman.ru/forum/?sort=1d";
static WORUM_TOP_THREADS_WEEK: &str = "https://woman.ru/forum/?sort=7d";
static WORUM_TOP_THREADS_MONTH: &str = "https://woman.ru/forum/?sort=30d";
static WORUM_TOP_THREADS_EVER: &str = "https://woman.ru/forum/?sort=all";

static SELECTOR_THREAD: &str = ".list-item";
static SELECTOR_THREAD_TITLE: &str = ".list-item__title";
static SELECTOR_THREAD_LINK: &str = ".list-item__link";
static SELECTOR_THREAD_TEXT: &str = ".card_topic-start .card__comment";
static SELECTOR_THREAD_IMAGE: &str = ".card_topic-start .imagesList_itemImg";
static THREAD_TEXT_LIMIT: usize = 140;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting WorumTop bot");

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler));

    let bot = Bot::from_env();

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
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
    let img_selector = Selector::parse(SELECTOR_THREAD_IMAGE).unwrap();

    for node in html.select(&item_selector) {
        let mut thread = WorumThread::default();

        if let Some(title) = node.select(&title_selector).nth(0) {
            thread.title = title.text().collect();
        }

        if let Some(link) = node.select(&link_selector).nth(0) {
            let thread_path = link.value().attr("href").unwrap().to_string();
            thread.link = format!("https://woman.ru{}", thread_path);
        }

        if let Some(photo_url) = node.select(&img_selector).nth(0) {
            let photo_url = photo_url.value().attr("src").unwrap().to_string();
            let photo_url = format!("https:{}", photo_url);

            println!("photo url: {photo_url}");

            thread.photo_url = Some(photo_url);
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

async fn forum_get_random_subrubric_thread() -> Option<SubrubricThread> {
    let mut thread = SubrubricThread::default();

    let Some(content) = fetch_content("https://woman.ru/forum").await else {
        return None
    };

    let html = Html::parse_document(&content);

    let relations_rubric_selector =
        Selector::parse("a.sidebar__all-subrubrics-link[href^=\"/relations/\"]").unwrap();

    let mut nodes = html.select(&relations_rubric_selector);
    let nodes_count = nodes.clone().count();

    if nodes_count == 0 {
        println!("zero relations nodes");

        return None
    }

    let rand_rubric_idx = random::<usize>() % nodes_count;
    let rand_rubric_node = nodes.nth(rand_rubric_idx).unwrap();

    let rubric_path = rand_rubric_node.attr("href").unwrap().to_string();
    let rubric_section = rand_rubric_node.attr("data-section").unwrap().to_string();
    let rubric_title = rand_rubric_node.text().collect::<String>();

    thread.subrubric = rubric_title;
    thread.section = rubric_section;

    let rubric_url = format!("https://woman.ru{}", rubric_path);
    thread.subrubric_link = rubric_url;

    Some(thread)
}

async fn get_answer_top(
    command: Command,
    args: Vec<&str>,
) -> Option<String> {
    let threads_url = match command {
        Command::Top => WORUM_TOP_THREADS_DAY,
        Command::Week => WORUM_TOP_THREADS_WEEK,
        Command::Month => WORUM_TOP_THREADS_MONTH,
        Command::Ever => WORUM_TOP_THREADS_EVER,
        _ => return None,
    };

    let mut count: usize = 1;

    if args.len() > 1 {
        if let Ok(arg_count) = args[1].parse::<usize>() {
            count = if arg_count > 5 {
                5
            } else if arg_count < 1 {
                1
            } else {
                arg_count
            };
        } else {
            println!("failed to parse {}", args[1]);
        }
    }

    let Some(threads) = forum_get_threads(threads_url).await else {
        return None
    };

    if threads.len() == 0 {
        log::error!("no threads");
        return None
    }

    let mut answer = String::from("");

    for n in 0..count {
        let thread = &threads[n];
        let title = &thread.title;
        let link = &thread.link;

        let Some(text) = forum_get_thread_text(link).await else {
            return None
        };

        let topic = html::link(link, title);
        let mut text = text;

        if text.len() > THREAD_TEXT_LIMIT {
            text = text.chars()
                .take(THREAD_TEXT_LIMIT)
                .collect::<String>();
            text += "…";
        }

        let text = html::italic(&text);
        let thread_block = format!(
            "Топ-{} тема:\n{}\n\n{}\n\n", n + 1, topic, text);

        let thread_block = thread_block.as_str();

        answer += &thread_block;
    }

    Some(answer)
}

async fn get_answer_rand_rubric() -> Option<String> {
    let Some(rand_thread) = forum_get_random_subrubric_thread().await else {
        return None
    };

    let Some(rand_thread) = fill_subrubric_thread(rand_thread).await else {
        return None
    };

    let Some(text) = forum_get_thread_text(&rand_thread.link).await else {
        return None
    };

    let mut text = text;

    let msg_title = format!(
        "{} | {}",
        html::bold(&rand_thread.section),
        rand_thread.subrubric,
    );

    let msg_subtitle = html::link(&rand_thread.link, &rand_thread.title);
    
    if text.len() > THREAD_TEXT_LIMIT {
        text = text.chars()
            .take(THREAD_TEXT_LIMIT)
            .collect::<String>();
        text += "…";
    }

    let text = html::italic(&text);

    let answer = format!("{}\n{}\n\n{}", msg_title, msg_subtitle, text);

    Some(answer)
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut args = Vec::<&str>::new();

    if let Some(msg_text) = msg.text() {
        args = msg_text.trim().split(" ").collect();
    }

    let cmd = BotCommands::parse(msg.text().unwrap(), me.username()).unwrap();

    let answer = match cmd {
        Command::Top => get_answer_top(cmd, args).await,
        Command::Week => get_answer_top(cmd, args).await,
        Command::Month => get_answer_top(cmd, args).await,
        Command::Ever => get_answer_top(cmd, args).await,
        Command::Rubric => get_answer_rand_rubric().await,
    };
        
    bot.send_message(msg.chat.id, answer.unwrap())
        .parse_mode(ParseMode::Html)
        .send()
        .await?;

    Ok(())
}

async fn fill_subrubric_thread(thread: SubrubricThread) -> Option<SubrubricThread> {
    let mut thread = thread;

    let Some(content) = fetch_content(&thread.subrubric_link).await else {
        return None
    };

    let html = Html::parse_document(&content);

    let threads_selector = Selector::parse(".list_forum-knowledge .list-item__link").unwrap();

    println!("subrubic link: {}", thread.subrubric_link);

    let mut nodes = html.select(&threads_selector);
    println!("nodes count: {}", nodes.clone().count());

    let rand_thread_idx = random::<usize>() % nodes.clone().count();
    let rand_thread_node = nodes.nth(rand_thread_idx).unwrap();

    let thread_url = rand_thread_node.attr("href").unwrap().to_string();
    thread.link = format!("https://woman.ru{}", thread_url);

    let thread_title_selector = Selector::parse(".list-item__title").unwrap();

    if let Some(title) = rand_thread_node.select(&thread_title_selector).nth(0) {
        thread.title = title.text().collect();
    }

    Some(thread)
}
