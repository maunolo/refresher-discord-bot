//! Admin page with a made-up camp and no Discord: `cargo run -p refresher-web --example demo --features demo`.
//! Run from the repo root so `web/dist` resolves. `DEMO_BIND` changes the address (default 127.0.0.1:8090);
//! behind a proxy, `WEB_PUBLIC_URL` is the address browsers open.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind = std::env::var("DEMO_BIND").unwrap_or_else(|_| "127.0.0.1:8090".to_string());
    let public_url = std::env::var("WEB_PUBLIC_URL").unwrap_or_else(|_| format!("http://{bind}"));
    refresher_web::demo::run(&bind, public_url.trim_end_matches('/'), "web/dist").await
}
