use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct MarketCommentRecord {
    pub id: Uuid,
    pub market_id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewMarketCommentRecord {
    pub id: Uuid,
    pub market_id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub body: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketCommentWithAuthorRecord {
    pub id: Uuid,
    pub market_id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub like_count: i64,
    pub reply_count: i64,
}
