use anyhow::Result;
use rand::RngCore;
use refresher_core::db::now;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

pub const COOKIE_NAME: &str = "refresher_session";

/// Admins sign in again after this long, which also refreshes which servers they manage.
pub const SESSION_SECONDS: i64 = 12 * 60 * 60;

/// A server where the signed-in user has Manage Server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminGuild {
    pub id: u64,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub user_id: u64,
    pub display_name: String,
    pub avatar: Option<String>,
    pub admin_guilds: Vec<AdminGuild>,
}

impl Session {
    pub fn is_admin_of(&self, guild_id: u64) -> bool {
        self.admin_guilds.iter().any(|g| g.id == guild_id)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 32 random bytes as hex, for session tokens and the login `state` check.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex(&bytes)
}

fn hash_token(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// Stores a new session and returns the token for the cookie. Only its hash is saved,
/// so a copy of the database can't be used to sign in.
pub async fn create(pool: &SqlitePool, session: &Session) -> Result<String> {
    let token = random_token();
    let ts = now();
    sqlx::query("DELETE FROM web_sessions WHERE expires_at <= ?")
        .bind(ts)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO web_sessions (token_hash, user_id, display_name, avatar, admin_guilds, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(hash_token(&token))
    .bind(session.user_id as i64)
    .bind(&session.display_name)
    .bind(&session.avatar)
    .bind(serde_json::to_string(&session.admin_guilds)?)
    .bind(ts)
    .bind(ts + SESSION_SECONDS)
    .execute(pool)
    .await?;
    Ok(token)
}

pub async fn load(pool: &SqlitePool, token: &str) -> Result<Option<Session>> {
    let row: Option<(i64, String, Option<String>, String)> = sqlx::query_as(
        "SELECT user_id, display_name, avatar, admin_guilds FROM web_sessions
         WHERE token_hash = ? AND expires_at > ?",
    )
    .bind(hash_token(token))
    .bind(now())
    .fetch_optional(pool)
    .await?;
    let Some((user_id, display_name, avatar, admin_guilds)) = row else {
        return Ok(None);
    };
    Ok(Some(Session {
        user_id: user_id as u64,
        display_name,
        avatar,
        admin_guilds: serde_json::from_str(&admin_guilds)?,
    }))
}

pub async fn delete(pool: &SqlitePool, token: &str) -> Result<()> {
    sqlx::query("DELETE FROM web_sessions WHERE token_hash = ?")
        .bind(hash_token(token))
        .execute(pool)
        .await?;
    Ok(())
}
