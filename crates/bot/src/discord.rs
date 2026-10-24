use std::sync::Arc;

use serenity::all::{Cache, Context, Http};

/// The bot's Discord connection. Unlike an event's `Context`, the web admin can hold one too.
#[derive(Clone)]
pub struct Discord {
    pub http: Arc<Http>,
    pub cache: Arc<Cache>,
}

impl From<&Context> for Discord {
    fn from(ctx: &Context) -> Self {
        Self {
            http: ctx.http.clone(),
            cache: ctx.cache.clone(),
        }
    }
}
