//! Admin page with a made-up camp and no Discord: `cargo run -p refresher-web --example demo --features demo`.
//! Run from the repo root so `web/dist` resolves. `DEMO_BIND` changes the address (default 127.0.0.1:8090).

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind = std::env::var("DEMO_BIND").unwrap_or_else(|_| "127.0.0.1:8090".to_string());
    refresher_web::demo::run(&bind, "web/dist").await
}
