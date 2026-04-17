use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::module::market::schema::{EventOnChainResponse, EventResponse, MarketResponse};

#[derive(Debug, Deserialize)]
pub struct CreateMarketCommentRequest {
    pub comment: CreateMarketCommentFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct CreateMarketCommentFieldsRequest {
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct MarketCommentAuthorResponse {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MarketCommentResponse {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_comment_id: Option<Uuid>,
    pub body: String,
    pub author: MarketCommentAuthorResponse,
    pub like_count: u64,
    pub reply_count: u64,
    pub replies: Vec<MarketCommentResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketCommentsResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub comments: Vec<MarketCommentResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketCommentWriteResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub comment: MarketCommentResponse,
}

#[derive(Debug, Serialize)]
pub struct MarketCommentLikeResponse {
    pub comment_id: Uuid,
    pub market_id: Uuid,
    pub like_count: u64,
    pub liked: bool,
    pub updated_at: DateTime<Utc>,
}
