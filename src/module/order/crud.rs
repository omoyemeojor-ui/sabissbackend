use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        auth::error::AuthError,
        market::model::{MarketEventRecord, MarketRecord},
        order::model::{
            MarketOrderFillRecord, MarketOrderRecord, NewMarketOrderRecord,
            NewUserMarketTradeRecord, UserMarketTradeRecord, UserTradeHistoryRecord,
        },
    },
};

mod sql {
    pub const INSERT_MARKET_ORDER: &str = r#"
        INSERT INTO market_orders (
            id,
            user_id,
            market_id,
            event_id,
            wallet_address,
            account_kind,
            condition_id,
            outcome_index,
            side,
            price_bps,
            amount,
            filled_amount,
            remaining_amount,
            expiry_epoch_seconds,
            salt,
            signature,
            order_hash,
            order_digest,
            status
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19
        )
        RETURNING
            id,
            user_id,
            market_id,
            event_id,
            wallet_address,
            account_kind,
            condition_id,
            outcome_index,
            side,
            price_bps,
            amount,
            filled_amount,
            remaining_amount,
            expiry_epoch_seconds,
            salt,
            signature,
            order_hash,
            order_digest,
            status,
            cancelled_at,
            created_at,
            updated_at
    "#;

    pub const GET_MARKET_ORDER_BY_ID_AND_USER_ID: &str = r#"
        SELECT
            id,
            user_id,
            market_id,
            event_id,
            wallet_address,
            account_kind,
            condition_id,
            outcome_index,
            side,
            price_bps,
            amount,
            filled_amount,
            remaining_amount,
            expiry_epoch_seconds,
            salt,
            signature,
            order_hash,
            order_digest,
            status,
            cancelled_at,
            created_at,
            updated_at
        FROM market_orders
        WHERE id = $1 AND user_id = $2
    "#;

    pub const LIST_MARKET_ORDERS_BY_USER_ID: &str = r#"
        SELECT
            id,
            user_id,
            market_id,
            event_id,
            wallet_address,
            account_kind,
            condition_id,
            outcome_index,
            side,
            price_bps,
            amount,
            filled_amount,
            remaining_amount,
            expiry_epoch_seconds,
            salt,
            signature,
            order_hash,
            order_digest,
            status,
            cancelled_at,
            created_at,
            updated_at
        FROM market_orders
        WHERE user_id = $1
        ORDER BY created_at DESC
    "#;

    pub const LIST_ACTIVE_MARKET_ORDERS_BY_MARKET_ID: &str = r#"
        SELECT
            id,
            user_id,
            market_id,
            event_id,
            wallet_address,
            account_kind,
            condition_id,
            outcome_index,
            side,
            price_bps,
            amount,
            filled_amount,
            remaining_amount,
            expiry_epoch_seconds,
            salt,
            signature,
            order_hash,
            order_digest,
            status,
            cancelled_at,
            created_at,
            updated_at
        FROM market_orders
        WHERE
            market_id = $1
            AND status IN ('open', 'partially_filled')
            AND (
                expiry_epoch_seconds IS NULL
                OR expiry_epoch_seconds = 0
                OR expiry_epoch_seconds >= EXTRACT(EPOCH FROM NOW())::BIGINT
            )
        ORDER BY
            CASE side WHEN 'buy' THEN price_bps END DESC NULLS LAST,
            CASE side WHEN 'sell' THEN price_bps END ASC NULLS LAST,
            created_at ASC
    "#;

    pub const CANCEL_MARKET_ORDER_BY_ID_AND_USER_ID: &str = r#"
        UPDATE market_orders
        SET
            status = 'cancelled',
            cancelled_at = NOW(),
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING
            id,
            user_id,
            market_id,
            event_id,
            wallet_address,
            account_kind,
            condition_id,
            outcome_index,
            side,
            price_bps,
            amount,
            filled_amount,
            remaining_amount,
            expiry_epoch_seconds,
            salt,
            signature,
            order_hash,
            order_digest,
            status,
            cancelled_at,
            created_at,
            updated_at
    "#;

    pub const LIST_MARKET_ORDER_FILLS_BY_MARKET_ID: &str = r#"
        SELECT
            id,
            market_id,
            event_id,
            condition_id,
            match_type,
            buy_order_id,
            sell_order_id,
            yes_order_id,
            no_order_id,
            outcome_index,
            fill_amount,
            collateral_amount,
            yes_price_bps,
            no_price_bps,
            tx_hash,
            created_at
        FROM market_order_fills
        WHERE market_id = $1
        ORDER BY created_at DESC
        LIMIT $2
    "#;

    pub const LIST_MARKETS_WITH_CONDITION_IDS: &str = r#"
        SELECT
            id,
            event_db_id,
            slug,
            label,
            question,
            question_id,
            condition_id,
            market_type,
            outcome_count,
            outcomes,
            end_time,
            sort_order,
            publication_status,
            trading_status,
            metadata_hash,
            oracle_address,
            created_at,
            updated_at
        FROM markets
        WHERE condition_id IS NOT NULL
        ORDER BY end_time ASC, sort_order ASC, created_at ASC
    "#;

    pub const INSERT_USER_MARKET_TRADE: &str = r#"
        INSERT INTO user_market_trade_history (
            user_id,
            market_id,
            event_id,
            wallet_address,
            execution_source,
            action,
            outcome_index,
            price_bps,
            token_amount,
            usdc_amount,
            tx_hash
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING
            id,
            user_id,
            market_id,
            event_id,
            wallet_address,
            execution_source,
            action,
            outcome_index,
            price_bps,
            token_amount,
            usdc_amount,
            tx_hash,
            created_at
    "#;

    pub const LIST_USER_TRADE_HISTORY_BY_USER_ID: &str = r#"
        SELECT
            history_key,
            market_id,
            event_id,
            wallet_address,
            execution_source,
            action,
            outcome_index,
            price_bps,
            token_amount,
            usdc_amount,
            tx_hash,
            executed_at
        FROM (
            SELECT
                'market_trade:' || id::TEXT AS history_key,
                market_id,
                event_id,
                wallet_address,
                execution_source,
                action,
                outcome_index,
                price_bps,
                token_amount,
                usdc_amount,
                tx_hash,
                created_at AS executed_at
            FROM user_market_trade_history
            WHERE user_id = $1

            UNION ALL

            SELECT
                'order_fill:' || f.id::TEXT || ':' || o.id::TEXT AS history_key,
                f.market_id,
                f.event_id,
                o.wallet_address,
                'order_fill' AS execution_source,
                o.side AS action,
                o.outcome_index,
                CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END AS price_bps,
                f.fill_amount AS token_amount,
                TRUNC(
                    (
                        f.fill_amount::NUMERIC
                        * CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END::NUMERIC
                    ) / 10000
                )::TEXT AS usdc_amount,
                f.tx_hash,
                f.created_at AS executed_at
            FROM market_order_fills f
            INNER JOIN market_orders o ON o.id = f.buy_order_id
            WHERE o.user_id = $1

            UNION ALL

            SELECT
                'order_fill:' || f.id::TEXT || ':' || o.id::TEXT AS history_key,
                f.market_id,
                f.event_id,
                o.wallet_address,
                'order_fill' AS execution_source,
                o.side AS action,
                o.outcome_index,
                CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END AS price_bps,
                f.fill_amount AS token_amount,
                TRUNC(
                    (
                        f.fill_amount::NUMERIC
                        * CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END::NUMERIC
                    ) / 10000
                )::TEXT AS usdc_amount,
                f.tx_hash,
                f.created_at AS executed_at
            FROM market_order_fills f
            INNER JOIN market_orders o ON o.id = f.sell_order_id
            WHERE o.user_id = $1

            UNION ALL

            SELECT
                'order_fill:' || f.id::TEXT || ':' || o.id::TEXT AS history_key,
                f.market_id,
                f.event_id,
                o.wallet_address,
                'order_fill' AS execution_source,
                o.side AS action,
                o.outcome_index,
                CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END AS price_bps,
                f.fill_amount AS token_amount,
                TRUNC(
                    (
                        f.fill_amount::NUMERIC
                        * CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END::NUMERIC
                    ) / 10000
                )::TEXT AS usdc_amount,
                f.tx_hash,
                f.created_at AS executed_at
            FROM market_order_fills f
            INNER JOIN market_orders o ON o.id = f.yes_order_id
            WHERE o.user_id = $1

            UNION ALL

            SELECT
                'order_fill:' || f.id::TEXT || ':' || o.id::TEXT AS history_key,
                f.market_id,
                f.event_id,
                o.wallet_address,
                'order_fill' AS execution_source,
                o.side AS action,
                o.outcome_index,
                CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END AS price_bps,
                f.fill_amount AS token_amount,
                TRUNC(
                    (
                        f.fill_amount::NUMERIC
                        * CASE WHEN o.outcome_index = 0 THEN f.yes_price_bps ELSE f.no_price_bps END::NUMERIC
                    ) / 10000
                )::TEXT AS usdc_amount,
                f.tx_hash,
                f.created_at AS executed_at
            FROM market_order_fills f
            INNER JOIN market_orders o ON o.id = f.no_order_id
            WHERE o.user_id = $1
        ) history
        ORDER BY executed_at DESC, history_key DESC
    "#;
}

pub async fn insert_market_order(
    pool: &DbPool,
    order: &NewMarketOrderRecord,
) -> Result<MarketOrderRecord, sqlx::Error> {
    sqlx::query_as::<_, MarketOrderRecord>(sql::INSERT_MARKET_ORDER)
        .bind(order.id)
        .bind(order.user_id)
        .bind(order.market_id)
        .bind(order.event_id)
        .bind(&order.wallet_address)
        .bind(&order.account_kind)
        .bind(&order.condition_id)
        .bind(order.outcome_index)
        .bind(&order.side)
        .bind(order.price_bps)
        .bind(&order.amount)
        .bind(&order.filled_amount)
        .bind(&order.remaining_amount)
        .bind(order.expiry_epoch_seconds)
        .bind(&order.salt)
        .bind(&order.signature)
        .bind(&order.order_hash)
        .bind(&order.order_digest)
        .bind(&order.status)
        .fetch_one(pool)
        .await
}

pub async fn get_market_order_by_id_and_user_id(
    pool: &DbPool,
    order_id: Uuid,
    user_id: Uuid,
) -> Result<Option<MarketOrderRecord>, AuthError> {
    sqlx::query_as::<_, MarketOrderRecord>(sql::GET_MARKET_ORDER_BY_ID_AND_USER_ID)
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_market_orders_by_user_id(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Vec<MarketOrderRecord>, AuthError> {
    sqlx::query_as::<_, MarketOrderRecord>(sql::LIST_MARKET_ORDERS_BY_USER_ID)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_active_market_orders_by_market_id(
    pool: &DbPool,
    market_id: Uuid,
) -> Result<Vec<MarketOrderRecord>, AuthError> {
    sqlx::query_as::<_, MarketOrderRecord>(sql::LIST_ACTIVE_MARKET_ORDERS_BY_MARKET_ID)
        .bind(market_id)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn cancel_market_order_by_id_and_user_id(
    pool: &DbPool,
    order_id: Uuid,
    user_id: Uuid,
) -> Result<MarketOrderRecord, AuthError> {
    sqlx::query_as::<_, MarketOrderRecord>(sql::CANCEL_MARKET_ORDER_BY_ID_AND_USER_ID)
        .bind(order_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_market_order_fills_by_market_id(
    pool: &DbPool,
    market_id: Uuid,
    limit: i64,
) -> Result<Vec<MarketOrderFillRecord>, AuthError> {
    sqlx::query_as::<_, MarketOrderFillRecord>(sql::LIST_MARKET_ORDER_FILLS_BY_MARKET_ID)
        .bind(market_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_markets_by_ids(
    pool: &DbPool,
    market_ids: &[Uuid],
) -> Result<Vec<MarketRecord>, AuthError> {
    if market_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            id,
            event_db_id,
            slug,
            label,
            question,
            question_id,
            condition_id,
            market_type,
            outcome_count,
            outcomes,
            end_time,
            sort_order,
            publication_status,
            trading_status,
            metadata_hash,
            oracle_address,
            created_at,
            updated_at
        FROM markets
        WHERE id IN ("#,
    );

    let mut separated = builder.separated(", ");
    for market_id in market_ids {
        separated.push_bind(market_id);
    }
    separated.push_unseparated(")");
    builder.push(" ORDER BY end_time ASC, sort_order ASC, created_at ASC");

    builder
        .build_query_as::<MarketRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_market_events_by_ids(
    pool: &DbPool,
    event_ids: &[Uuid],
) -> Result<Vec<MarketEventRecord>, AuthError> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            id,
            title,
            slug,
            category_slug,
            subcategory_slug,
            tag_slugs,
            image_url,
            summary_text,
            rules_text,
            context_text,
            additional_context,
            resolution_sources,
            resolution_timezone,
            starts_at,
            sort_at,
            featured,
            breaking,
            searchable,
            visible,
            hide_resolved_by_default,
            group_key,
            series_key,
            event_id,
            group_id,
            series_id,
            neg_risk,
            oracle_address,
            publication_status,
            published_tx_hash,
            created_by_user_id,
            created_at,
            updated_at
        FROM market_events
        WHERE id IN ("#,
    );

    let mut separated = builder.separated(", ");
    for event_id in event_ids {
        separated.push_bind(event_id);
    }
    separated.push_unseparated(")");
    builder.push(" ORDER BY sort_at NULLS LAST, created_at ASC");

    builder
        .build_query_as::<MarketEventRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_markets_with_condition_ids(pool: &DbPool) -> Result<Vec<MarketRecord>, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::LIST_MARKETS_WITH_CONDITION_IDS)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn insert_user_market_trade(
    pool: &DbPool,
    trade: &NewUserMarketTradeRecord,
) -> Result<UserMarketTradeRecord, AuthError> {
    sqlx::query_as::<_, UserMarketTradeRecord>(sql::INSERT_USER_MARKET_TRADE)
        .bind(trade.user_id)
        .bind(trade.market_id)
        .bind(trade.event_id)
        .bind(&trade.wallet_address)
        .bind(&trade.execution_source)
        .bind(&trade.action)
        .bind(trade.outcome_index)
        .bind(trade.price_bps)
        .bind(&trade.token_amount)
        .bind(&trade.usdc_amount)
        .bind(trade.tx_hash.as_deref())
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_user_trade_history_by_user_id(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Vec<UserTradeHistoryRecord>, AuthError> {
    sqlx::query_as::<_, UserTradeHistoryRecord>(sql::LIST_USER_TRADE_HISTORY_BY_USER_ID)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}
