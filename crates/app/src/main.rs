use anyhow::Result;
use refresher_core::{config::Config, db};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    tokio::fs::create_dir_all("data").await?;
    let db = db::connect(&config.database_url).await?;
    let bot = refresher_bot::Bot::new(config.clone(), db.clone()).await?;
    let link = bot.link();
    tokio::try_join!(bot.run(), refresher_web::run(config.web, db, link))?;
    Ok(())
}
