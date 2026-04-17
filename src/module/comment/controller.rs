use axum::{
    Json,
    extract::{Extension, Path, State},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{auth::error::AuthError, comment::schema::*},
    service::{
        comment::{
            create_market_comment_entry,
            create_market_comment_reply as create_market_comment_reply_entry, get_market_comments,
            like_market_comment, unlike_market_comment,
        },
        jwt::AuthenticatedUser,
    },
};

pub async fn market_comments(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketCommentsResponse>, AuthError> {
    Ok(Json(get_market_comments(&state, market_id).await?))
}

pub async fn create_market_comment(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<CreateMarketCommentRequest>,
) -> Result<Json<MarketCommentWriteResponse>, AuthError> {
    Ok(Json(
        create_market_comment_entry(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn create_market_comment_reply(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path((market_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CreateMarketCommentRequest>,
) -> Result<Json<MarketCommentWriteResponse>, AuthError> {
    Ok(Json(
        create_market_comment_reply_entry(
            &state,
            authenticated_user,
            market_id,
            comment_id,
            payload,
        )
        .await?,
    ))
}

pub async fn like_comment(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<MarketCommentLikeResponse>, AuthError> {
    Ok(Json(
        like_market_comment(&state, authenticated_user, comment_id).await?,
    ))
}

pub async fn unlike_comment(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<MarketCommentLikeResponse>, AuthError> {
    Ok(Json(
        unlike_market_comment(&state, authenticated_user, comment_id).await?,
    ))
}
