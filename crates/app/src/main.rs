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

    match config.web {
        Some(web) => {
            let link = bot.link();
            tokio::try_join!(bot.run(), refresher_web::run(web, db, link))?;
        }
        None => {
            tracing::info!("web admin off: set DISCORD_CLIENT_ID and DISCORD_CLIENT_SECRET to turn it on");
            bot.run().await?;
        }
    }
    Ok(())
}
