use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        auth::error::AuthError,
        comment::model::{
            MarketCommentRecord, MarketCommentWithAuthorRecord, NewMarketCommentRecord,
        },
    },
};

mod sql {
    pub const INSERT_MARKET_COMMENT: &str = r#"
        INSERT INTO market_comments (
            id,
            market_id,
            event_id,
            user_id,
            parent_comment_id,
            body
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id,
            market_id,
            event_id,
            user_id,
            parent_comment_id,
            body,
            created_at,
            updated_at
    "#;

    pub const GET_MARKET_COMMENT_BY_ID: &str = r#"
        SELECT
            id,
            market_id,
            event_id,
            user_id,
            parent_comment_id,
            body,
            created_at,
            updated_at
        FROM market_comments
        WHERE id = $1
    "#;

    pub const GET_MARKET_COMMENT_WITH_AUTHOR_BY_ID: &str = r#"
        SELECT
            c.id,
            c.market_id,
            c.event_id,
            c.user_id,
            c.parent_comment_id,
            c.body,
            c.created_at,
            c.updated_at,
            u.username,
            u.display_name,
            u.avatar_url,
            COALESCE(l.like_count, 0) AS like_count,
            COALESCE(r.reply_count, 0) AS reply_count
        FROM market_comments c
        INNER JOIN users u ON u.id = c.user_id
        LEFT JOIN (
            SELECT comment_id, COUNT(*)::BIGINT AS like_count
            FROM market_comment_likes
            GROUP BY comment_id
        ) l ON l.comment_id = c.id
        LEFT JOIN (
            SELECT parent_comment_id, COUNT(*)::BIGINT AS reply_count
            FROM market_comments
            WHERE parent_comment_id IS NOT NULL
            GROUP BY parent_comment_id
        ) r ON r.parent_comment_id = c.id
        WHERE c.id = $1
    "#;

    pub const LIST_MARKET_COMMENTS_WITH_AUTHORS_BY_MARKET_ID: &str = r#"
        SELECT
            c.id,
            c.market_id,
            c.event_id,
            c.user_id,
            c.parent_comment_id,
            c.body,
            c.created_at,
            c.updated_at,
            u.username,
            u.display_name,
            u.avatar_url,
            COALESCE(l.like_count, 0) AS like_count,
            COALESCE(r.reply_count, 0) AS reply_count
        FROM market_comments c
        INNER JOIN users u ON u.id = c.user_id
        LEFT JOIN (
            SELECT comment_id, COUNT(*)::BIGINT AS like_count
            FROM market_comment_likes
            GROUP BY comment_id
        ) l ON l.comment_id = c.id
        LEFT JOIN (
            SELECT parent_comment_id, COUNT(*)::BIGINT AS reply_count
            FROM market_comments
            WHERE parent_comment_id IS NOT NULL
            GROUP BY parent_comment_id
        ) r ON r.parent_comment_id = c.id
        WHERE c.market_id = $1
        ORDER BY c.created_at DESC
        LIMIT $2
    "#;

    pub const INSERT_MARKET_COMMENT_LIKE: &str = r#"
        INSERT INTO market_comment_likes (
            comment_id,
            user_id
        )
        VALUES ($1, $2)
        ON CONFLICT (comment_id, user_id) DO NOTHING
    "#;

    pub const DELETE_MARKET_COMMENT_LIKE: &str = r#"
        DELETE FROM market_comment_likes
        WHERE comment_id = $1
          AND user_id = $2
    "#;

    pub const COUNT_MARKET_COMMENT_LIKES: &str = r#"
        SELECT COUNT(*)::BIGINT
        FROM market_comment_likes
        WHERE comment_id = $1
    "#;
}

pub async fn insert_market_comment(
    pool: &DbPool,
    comment: &NewMarketCommentRecord,
) -> Result<MarketCommentRecord, AuthError> {
    sqlx::query_as::<_, MarketCommentRecord>(sql::INSERT_MARKET_COMMENT)
        .bind(comment.id)
        .bind(comment.market_id)
        .bind(comment.event_id)
        .bind(comment.user_id)
        .bind(comment.parent_comment_id)
        .bind(&comment.body)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_market_comment_by_id(
    pool: &DbPool,
    comment_id: Uuid,
) -> Result<Option<MarketCommentRecord>, AuthError> {
    sqlx::query_as::<_, MarketCommentRecord>(sql::GET_MARKET_COMMENT_BY_ID)
        .bind(comment_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_market_comment_with_author_by_id(
    pool: &DbPool,
    comment_id: Uuid,
) -> Result<Option<MarketCommentWithAuthorRecord>, AuthError> {
    sqlx::query_as::<_, MarketCommentWithAuthorRecord>(sql::GET_MARKET_COMMENT_WITH_AUTHOR_BY_ID)
        .bind(comment_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_market_comments_with_authors_by_market_id(
    pool: &DbPool,
    market_id: Uuid,
    limit: i64,
) -> Result<Vec<MarketCommentWithAuthorRecord>, AuthError> {
    sqlx::query_as::<_, MarketCommentWithAuthorRecord>(
        sql::LIST_MARKET_COMMENTS_WITH_AUTHORS_BY_MARKET_ID,
    )
    .bind(market_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AuthError::from)
}

pub async fn insert_market_comment_like(
    pool: &DbPool,
    comment_id: Uuid,
    user_id: Uuid,
) -> Result<(), AuthError> {
    sqlx::query(sql::INSERT_MARKET_COMMENT_LIKE)
        .bind(comment_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(AuthError::from)
}

pub async fn delete_market_comment_like(
    pool: &DbPool,
    comment_id: Uuid,
    user_id: Uuid,
) -> Result<(), AuthError> {
    sqlx::query(sql::DELETE_MARKET_COMMENT_LIKE)
        .bind(comment_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(AuthError::from)
}

pub async fn count_market_comment_likes(pool: &DbPool, comment_id: Uuid) -> Result<i64, AuthError> {
    sqlx::query_scalar::<_, i64>(sql::COUNT_MARKET_COMMENT_LIKES)
        .bind(comment_id)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}
