use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        comment::{
            crud,
            model::{MarketCommentWithAuthorRecord, NewMarketCommentRecord},
            schema::*,
        },
    },
    service::{
        jwt::AuthenticatedUser, liquidity::view::build_market_response,
        market::load_public_market_context,
    },
};

const DEFAULT_COMMENTS_LIMIT: i64 = 100;
const MAX_COMMENT_BODY_LEN: usize = 2_000;

pub async fn get_market_comments(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketCommentsResponse, AuthError> {
    let context = load_public_market_context(state, market_id).await?;
    let comments = crud::list_market_comments_with_authors_by_market_id(
        &state.db,
        market_id,
        DEFAULT_COMMENTS_LIMIT,
    )
    .await?;

    Ok(MarketCommentsResponse {
        event: (&context.event).into(),
        on_chain: (&context.event).into(),
        market: build_market_response(state, &context.market).await?,
        comments: build_market_comment_tree(comments)?,
    })
}

pub async fn create_market_comment_entry(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: CreateMarketCommentRequest,
) -> Result<MarketCommentWriteResponse, AuthError> {
    let context = load_public_market_context(state, market_id).await?;
    let body = normalize_comment_body(&payload.comment.body)?;
    let record = crud::insert_market_comment(
        &state.db,
        &NewMarketCommentRecord {
            id: Uuid::new_v4(),
            market_id: context.market.id,
            event_id: context.event.id,
            user_id: authenticated_user.user_id,
            parent_comment_id: None,
            body,
        },
    )
    .await?;
    let comment = crud::get_market_comment_with_author_by_id(&state.db, record.id)
        .await?
        .ok_or_else(|| AuthError::internal("comment lookup failed", record.id))?;

    Ok(MarketCommentWriteResponse {
        event: (&context.event).into(),
        on_chain: (&context.event).into(),
        market: build_market_response(state, &context.market).await?,
        comment: build_market_comment_response(&comment),
    })
}

pub async fn create_market_comment_reply(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    parent_comment_id: Uuid,
    payload: CreateMarketCommentRequest,
) -> Result<MarketCommentWriteResponse, AuthError> {
    let context = load_public_market_context(state, market_id).await?;
    let parent = crud::get_market_comment_by_id(&state.db, parent_comment_id)
        .await?
        .ok_or_else(|| AuthError::not_found("parent comment not found"))?;
    if parent.market_id != context.market.id {
        return Err(AuthError::bad_request(
            "parent comment does not belong to this market",
        ));
    }

    let body = normalize_comment_body(&payload.comment.body)?;
    let record = crud::insert_market_comment(
        &state.db,
        &NewMarketCommentRecord {
            id: Uuid::new_v4(),
            market_id: context.market.id,
            event_id: context.event.id,
            user_id: authenticated_user.user_id,
            parent_comment_id: Some(parent.id),
            body,
        },
    )
    .await?;
    let comment = crud::get_market_comment_with_author_by_id(&state.db, record.id)
        .await?
        .ok_or_else(|| AuthError::internal("reply lookup failed", record.id))?;

    Ok(MarketCommentWriteResponse {
        event: (&context.event).into(),
        on_chain: (&context.event).into(),
        market: build_market_response(state, &context.market).await?,
        comment: build_market_comment_response(&comment),
    })
}

pub async fn like_market_comment(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    comment_id: Uuid,
) -> Result<MarketCommentLikeResponse, AuthError> {
    let comment = crud::get_market_comment_by_id(&state.db, comment_id)
        .await?
        .ok_or_else(|| AuthError::not_found("comment not found"))?;
    crud::insert_market_comment_like(&state.db, comment_id, authenticated_user.user_id).await?;
    let like_count = crud::count_market_comment_likes(&state.db, comment_id).await?;

    Ok(MarketCommentLikeResponse {
        comment_id,
        market_id: comment.market_id,
        like_count: non_negative_count(like_count, "comment like count")?,
        liked: true,
        updated_at: Utc::now(),
    })
}

pub async fn unlike_market_comment(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    comment_id: Uuid,
) -> Result<MarketCommentLikeResponse, AuthError> {
    let comment = crud::get_market_comment_by_id(&state.db, comment_id)
        .await?
        .ok_or_else(|| AuthError::not_found("comment not found"))?;
    crud::delete_market_comment_like(&state.db, comment_id, authenticated_user.user_id).await?;
    let like_count = crud::count_market_comment_likes(&state.db, comment_id).await?;

    Ok(MarketCommentLikeResponse {
        comment_id,
        market_id: comment.market_id,
        like_count: non_negative_count(like_count, "comment like count")?,
        liked: false,
        updated_at: Utc::now(),
    })
}

fn build_market_comment_tree(
    comments: Vec<MarketCommentWithAuthorRecord>,
) -> Result<Vec<MarketCommentResponse>, AuthError> {
    let mut comments_by_parent = HashMap::<Option<Uuid>, Vec<MarketCommentWithAuthorRecord>>::new();
    for comment in comments {
        comments_by_parent
            .entry(comment.parent_comment_id)
            .or_default()
            .push(comment);
    }

    let mut roots = comments_by_parent.remove(&None).unwrap_or_default();
    roots.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    roots
        .into_iter()
        .map(|comment| build_market_comment_response_recursive(comment, &mut comments_by_parent))
        .collect()
}

fn build_market_comment_response_recursive(
    comment: MarketCommentWithAuthorRecord,
    comments_by_parent: &mut HashMap<Option<Uuid>, Vec<MarketCommentWithAuthorRecord>>,
) -> Result<MarketCommentResponse, AuthError> {
    let mut replies = comments_by_parent
        .remove(&Some(comment.id))
        .unwrap_or_default();
    replies.sort_by(|left, right| left.created_at.cmp(&right.created_at));

    let replies = replies
        .into_iter()
        .map(|reply| build_market_comment_response_recursive(reply, comments_by_parent))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(build_market_comment_response_with_replies(
        &comment, replies,
    )?)
}

fn build_market_comment_response(comment: &MarketCommentWithAuthorRecord) -> MarketCommentResponse {
    build_market_comment_response_with_replies(comment, Vec::new()).unwrap_or_else(|_| {
        MarketCommentResponse {
            id: comment.id,
            parent_comment_id: comment.parent_comment_id,
            body: comment.body.clone(),
            author: MarketCommentAuthorResponse {
                user_id: comment.user_id,
                username: comment.username.clone(),
                display_name: comment.display_name.clone(),
                avatar_url: comment.avatar_url.clone(),
            },
            like_count: 0,
            reply_count: 0,
            replies: Vec::new(),
            created_at: comment.created_at,
            updated_at: comment.updated_at,
        }
    })
}

fn build_market_comment_response_with_replies(
    comment: &MarketCommentWithAuthorRecord,
    replies: Vec<MarketCommentResponse>,
) -> Result<MarketCommentResponse, AuthError> {
    Ok(MarketCommentResponse {
        id: comment.id,
        parent_comment_id: comment.parent_comment_id,
        body: comment.body.clone(),
        author: MarketCommentAuthorResponse {
            user_id: comment.user_id,
            username: comment.username.clone(),
            display_name: comment.display_name.clone(),
            avatar_url: comment.avatar_url.clone(),
        },
        like_count: non_negative_count(comment.like_count, "comment like count")?,
        reply_count: non_negative_count(comment.reply_count, "comment reply count")?,
        replies,
        created_at: comment.created_at,
        updated_at: comment.updated_at,
    })
}

fn normalize_comment_body(raw: &str) -> Result<String, AuthError> {
    let body = raw.trim();
    if body.is_empty() {
        return Err(AuthError::bad_request("comment.body is required"));
    }
    if body.len() > MAX_COMMENT_BODY_LEN {
        return Err(AuthError::bad_request(format!(
            "comment.body must be at most {MAX_COMMENT_BODY_LEN} characters"
        )));
    }

    Ok(body.to_owned())
}

fn non_negative_count(raw: i64, field_name: &str) -> Result<u64, AuthError> {
    u64::try_from(raw).map_err(|error| {
        tracing::warn!(field_name, raw, ?error, "invalid negative comment count");
        AuthError::internal("invalid comment count", error)
    })
}
