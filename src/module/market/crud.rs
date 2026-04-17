use crate::{
    config::db::DbPool,
    module::{
        auth::error::AuthError,
        market::model::{
            CategorySummaryRecord, MarketAutoCreateSeriesRecord, MarketAutoResolutionConfigRecord,
            MarketConditionRecord, MarketEventNegRiskConfigRecord, MarketEventRecord,
            MarketPriceHistorySnapshotRecord, MarketPriceSnapshotRecord, MarketRecord,
            MarketResolutionRecord, MarketTradeStatsRecord, NewMarketAutoCreateSeriesRecord,
            NewMarketAutoResolutionConfigRecord, NewMarketEventNegRiskConfigRecord,
            NewMarketEventRecord, NewMarketRecord, NewMarketResolutionRecord,
            PendingMarketAutoResolutionRecord, PublicEventSummaryRecord, PublicMarketSummaryRecord,
            TagSummaryRecord,
        },
    },
};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

mod sql {
    pub const GET_MARKET_EVENT_BY_ID: &str = r#"
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
        WHERE id = $1
    "#;

    pub const INSERT_MARKET_EVENT: &str = r#"
        INSERT INTO market_events (
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
            created_by_user_id
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27, $28, $29, $30
        )
        RETURNING
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
    "#;

    pub const INSERT_MARKET: &str = r#"
        INSERT INTO markets (
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
            oracle_address
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING
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
    "#;

    pub const GET_MARKET_BY_ID: &str = r#"
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
        WHERE id = $1
    "#;

    pub const GET_MARKET_BY_SLUG: &str = r#"
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
        WHERE slug = $1
    "#;

    pub const GET_PUBLIC_MARKET_BY_ID: &str = r#"
        SELECT
            m.id,
            m.event_db_id,
            m.slug,
            m.label,
            m.question,
            m.question_id,
            m.condition_id,
            m.market_type,
            m.outcome_count,
            m.outcomes,
            m.end_time,
            m.sort_order,
            m.publication_status,
            m.trading_status,
            m.metadata_hash,
            m.oracle_address,
            m.created_at,
            m.updated_at
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.id = $1
            AND m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND e.visible = TRUE
    "#;

    pub const GET_PUBLIC_MARKET_BY_SLUG: &str = r#"
        SELECT
            m.id,
            m.event_db_id,
            m.slug,
            m.label,
            m.question,
            m.question_id,
            m.condition_id,
            m.market_type,
            m.outcome_count,
            m.outcomes,
            m.end_time,
            m.sort_order,
            m.publication_status,
            m.trading_status,
            m.metadata_hash,
            m.oracle_address,
            m.created_at,
            m.updated_at
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.slug = $1
            AND m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND e.visible = TRUE
    "#;

    pub const GET_PUBLIC_MARKET_BY_CONDITION_ID: &str = r#"
        SELECT
            m.id,
            m.event_db_id,
            m.slug,
            m.label,
            m.question,
            m.question_id,
            m.condition_id,
            m.market_type,
            m.outcome_count,
            m.outcomes,
            m.end_time,
            m.sort_order,
            m.publication_status,
            m.trading_status,
            m.metadata_hash,
            m.oracle_address,
            m.created_at,
            m.updated_at
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.condition_id = $1
            AND m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND e.visible = TRUE
    "#;

    pub const GET_PUBLIC_MARKET_EVENT_BY_ID: &str = r#"
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
        WHERE
            id = $1
            AND publication_status = 'published'
            AND visible = TRUE
    "#;

    pub const COUNT_PUBLIC_MARKETS_FOR_EVENT: &str = r#"
        SELECT COUNT(*)::BIGINT
        FROM markets
        WHERE
            event_db_id = $1
            AND publication_status = 'published'
    "#;

    pub const LIST_CATEGORY_SUMMARIES: &str = r#"
        SELECT
            e.category_slug AS slug,
            COUNT(DISTINCT e.id)::BIGINT AS event_count,
            COUNT(m.id)::BIGINT AS market_count,
            COUNT(DISTINCT CASE WHEN e.featured THEN e.id END)::BIGINT AS featured_event_count,
            COUNT(DISTINCT CASE WHEN e.breaking THEN e.id END)::BIGINT AS breaking_event_count
        FROM market_events e
        LEFT JOIN markets m
            ON m.event_db_id = e.id
            AND m.publication_status = 'published'
        WHERE
            e.publication_status = 'published'
            AND e.visible = TRUE
        GROUP BY e.category_slug
        ORDER BY market_count DESC, slug ASC
    "#;

    pub const GET_CATEGORY_SUMMARY_BY_SLUG: &str = r#"
        SELECT
            e.category_slug AS slug,
            COUNT(DISTINCT e.id)::BIGINT AS event_count,
            COUNT(m.id)::BIGINT AS market_count,
            COUNT(DISTINCT CASE WHEN e.featured THEN e.id END)::BIGINT AS featured_event_count,
            COUNT(DISTINCT CASE WHEN e.breaking THEN e.id END)::BIGINT AS breaking_event_count
        FROM market_events e
        LEFT JOIN markets m
            ON m.event_db_id = e.id
            AND m.publication_status = 'published'
        WHERE
            e.publication_status = 'published'
            AND e.visible = TRUE
            AND e.category_slug = $1
        GROUP BY e.category_slug
    "#;

    pub const LIST_TAG_SUMMARIES: &str = r#"
        SELECT
            tag.slug AS slug,
            COUNT(DISTINCT e.id)::BIGINT AS event_count,
            COUNT(m.id)::BIGINT AS market_count
        FROM market_events e
        CROSS JOIN LATERAL unnest(e.tag_slugs) AS tag(slug)
        LEFT JOIN markets m
            ON m.event_db_id = e.id
            AND m.publication_status = 'published'
        WHERE
            e.publication_status = 'published'
            AND e.visible = TRUE
        GROUP BY tag.slug
        ORDER BY market_count DESC, slug ASC
    "#;

    pub const GET_MARKET_RESOLUTION_BY_MARKET_ID: &str = r#"
        SELECT
            market_id,
            status,
            proposed_winning_outcome,
            final_winning_outcome,
            payout_vector_hash,
            proposed_by_user_id,
            proposed_at,
            dispute_deadline,
            notes,
            disputed_by_user_id,
            disputed_at,
            dispute_reason,
            finalized_by_user_id,
            finalized_at,
            emergency_resolved_by_user_id,
            emergency_resolved_at,
            created_at,
            updated_at
        FROM market_resolutions
        WHERE market_id = $1
    "#;

    pub const GET_MARKET_AUTO_RESOLUTION_CONFIG_BY_MARKET_ID: &str = r#"
        SELECT
            market_id,
            provider,
            product_id,
            start_time,
            start_price,
            start_price_captured_at,
            end_price,
            end_price_captured_at,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            last_error,
            created_at,
            updated_at
        FROM market_auto_resolution_configs
        WHERE market_id = $1
    "#;

    pub const UPSERT_MARKET_AUTO_CREATE_SERIES: &str = r#"
        INSERT INTO market_auto_create_series (
            id,
            provider,
            product_id,
            title_prefix,
            slug_prefix,
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
            start_time,
            cadence_seconds,
            market_duration_seconds,
            oracle_address,
            outcomes,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            featured,
            breaking,
            searchable,
            visible,
            hide_resolved_by_default,
            active,
            last_created_slot_start,
            created_by_user_id
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31
        )
        ON CONFLICT (slug_prefix) DO UPDATE
        SET
            provider = EXCLUDED.provider,
            product_id = EXCLUDED.product_id,
            title_prefix = EXCLUDED.title_prefix,
            category_slug = EXCLUDED.category_slug,
            subcategory_slug = EXCLUDED.subcategory_slug,
            tag_slugs = EXCLUDED.tag_slugs,
            image_url = EXCLUDED.image_url,
            summary_text = EXCLUDED.summary_text,
            rules_text = EXCLUDED.rules_text,
            context_text = EXCLUDED.context_text,
            additional_context = EXCLUDED.additional_context,
            resolution_sources = EXCLUDED.resolution_sources,
            resolution_timezone = EXCLUDED.resolution_timezone,
            start_time = EXCLUDED.start_time,
            cadence_seconds = EXCLUDED.cadence_seconds,
            market_duration_seconds = EXCLUDED.market_duration_seconds,
            oracle_address = EXCLUDED.oracle_address,
            outcomes = EXCLUDED.outcomes,
            up_outcome_index = EXCLUDED.up_outcome_index,
            down_outcome_index = EXCLUDED.down_outcome_index,
            tie_outcome_index = EXCLUDED.tie_outcome_index,
            featured = EXCLUDED.featured,
            breaking = EXCLUDED.breaking,
            searchable = EXCLUDED.searchable,
            visible = EXCLUDED.visible,
            hide_resolved_by_default = EXCLUDED.hide_resolved_by_default,
            active = EXCLUDED.active,
            created_by_user_id = EXCLUDED.created_by_user_id,
            updated_at = NOW()
        RETURNING
            id,
            provider,
            product_id,
            title_prefix,
            slug_prefix,
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
            start_time,
            cadence_seconds,
            market_duration_seconds,
            oracle_address,
            outcomes,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            featured,
            breaking,
            searchable,
            visible,
            hide_resolved_by_default,
            active,
            last_created_slot_start,
            created_by_user_id,
            created_at,
            updated_at
    "#;

    pub const LIST_ACTIVE_MARKET_AUTO_CREATE_SERIES: &str = r#"
        SELECT
            id,
            provider,
            product_id,
            title_prefix,
            slug_prefix,
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
            start_time,
            cadence_seconds,
            market_duration_seconds,
            oracle_address,
            outcomes,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            featured,
            breaking,
            searchable,
            visible,
            hide_resolved_by_default,
            active,
            last_created_slot_start,
            created_by_user_id,
            created_at,
            updated_at
        FROM market_auto_create_series
        WHERE active = TRUE
        ORDER BY start_time ASC, created_at ASC
    "#;

    pub const UPDATE_MARKET_AUTO_CREATE_SERIES_LAST_CREATED_SLOT_START: &str = r#"
        UPDATE market_auto_create_series
        SET
            last_created_slot_start = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            provider,
            product_id,
            title_prefix,
            slug_prefix,
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
            start_time,
            cadence_seconds,
            market_duration_seconds,
            oracle_address,
            outcomes,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            featured,
            breaking,
            searchable,
            visible,
            hide_resolved_by_default,
            active,
            last_created_slot_start,
            created_by_user_id,
            created_at,
            updated_at
    "#;

    pub const UPSERT_MARKET_AUTO_RESOLUTION_CONFIG: &str = r#"
        INSERT INTO market_auto_resolution_configs (
            market_id,
            provider,
            product_id,
            start_time,
            start_price,
            start_price_captured_at,
            end_price,
            end_price_captured_at,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            last_error
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (market_id) DO UPDATE
        SET
            provider = EXCLUDED.provider,
            product_id = EXCLUDED.product_id,
            start_time = EXCLUDED.start_time,
            start_price = EXCLUDED.start_price,
            start_price_captured_at = EXCLUDED.start_price_captured_at,
            end_price = EXCLUDED.end_price,
            end_price_captured_at = EXCLUDED.end_price_captured_at,
            up_outcome_index = EXCLUDED.up_outcome_index,
            down_outcome_index = EXCLUDED.down_outcome_index,
            tie_outcome_index = EXCLUDED.tie_outcome_index,
            last_error = EXCLUDED.last_error,
            updated_at = NOW()
        RETURNING
            market_id,
            provider,
            product_id,
            start_time,
            start_price,
            start_price_captured_at,
            end_price,
            end_price_captured_at,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            last_error,
            created_at,
            updated_at
    "#;

    pub const UPDATE_MARKET_AUTO_RESOLUTION_START_PRICE: &str = r#"
        UPDATE market_auto_resolution_configs
        SET
            start_price = $2,
            start_price_captured_at = $3,
            last_error = NULL,
            updated_at = NOW()
        WHERE market_id = $1
        RETURNING
            market_id,
            provider,
            product_id,
            start_time,
            start_price,
            start_price_captured_at,
            end_price,
            end_price_captured_at,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            last_error,
            created_at,
            updated_at
    "#;

    pub const UPDATE_MARKET_AUTO_RESOLUTION_END_PRICE: &str = r#"
        UPDATE market_auto_resolution_configs
        SET
            end_price = $2,
            end_price_captured_at = $3,
            last_error = NULL,
            updated_at = NOW()
        WHERE market_id = $1
        RETURNING
            market_id,
            provider,
            product_id,
            start_time,
            start_price,
            start_price_captured_at,
            end_price,
            end_price_captured_at,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            last_error,
            created_at,
            updated_at
    "#;

    pub const UPDATE_MARKET_AUTO_RESOLUTION_ERROR: &str = r#"
        UPDATE market_auto_resolution_configs
        SET
            last_error = $2,
            updated_at = NOW()
        WHERE market_id = $1
        RETURNING
            market_id,
            provider,
            product_id,
            start_time,
            start_price,
            start_price_captured_at,
            end_price,
            end_price_captured_at,
            up_outcome_index,
            down_outcome_index,
            tie_outcome_index,
            last_error,
            created_at,
            updated_at
    "#;

    pub const LIST_PENDING_MARKET_AUTO_RESOLUTION_RECORDS: &str = r#"
        SELECT
            m.id AS market_id,
            m.condition_id,
            m.question,
            m.outcomes,
            m.end_time,
            m.publication_status,
            m.trading_status,
            m.oracle_address,
            r.status AS resolution_status,
            r.dispute_deadline,
            c.provider,
            c.product_id,
            c.start_time,
            c.start_price,
            c.start_price_captured_at,
            c.end_price,
            c.end_price_captured_at,
            c.up_outcome_index,
            c.down_outcome_index,
            c.tie_outcome_index,
            c.last_error
        FROM market_auto_resolution_configs c
        INNER JOIN markets m ON m.id = c.market_id
        LEFT JOIN market_resolutions r ON r.market_id = m.id
        WHERE
            m.publication_status = 'published'
            AND m.condition_id IS NOT NULL
            AND m.trading_status <> 'resolved'
        ORDER BY m.end_time ASC, m.created_at ASC
    "#;

    pub const GET_EVENT_NEG_RISK_CONFIG_BY_EVENT_ID: &str = r#"
        SELECT
            event_id,
            registered,
            has_other,
            other_market_id,
            other_condition_id,
            registered_by_user_id,
            registered_at,
            created_at,
            updated_at
        FROM market_event_neg_risk_configs
        WHERE event_id = $1
    "#;

    pub const COUNT_MARKETS_FOR_EVENT: &str = r#"
        SELECT COUNT(*)::BIGINT
        FROM markets
        WHERE event_db_id = $1
    "#;

    pub const UPDATE_MARKET: &str = r#"
        UPDATE markets
        SET
            slug = $2,
            label = $3,
            question = $4,
            question_id = $5,
            outcome_count = $6,
            outcomes = $7,
            end_time = $8,
            sort_order = $9,
            oracle_address = $10,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
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
    "#;

    pub const UPDATE_MARKET_TRADING_STATUS: &str = r#"
        UPDATE markets
        SET
            trading_status = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
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
    "#;

    pub const UPDATE_MARKET_EVENT_PUBLICATION: &str = r#"
        UPDATE market_events
        SET
            publication_status = $2,
            published_tx_hash = $3,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
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
    "#;

    pub const UPDATE_MARKET_PUBLICATION: &str = r#"
        UPDATE markets
        SET
            publication_status = $2,
            condition_id = $3,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
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
    "#;

    pub const UPSERT_MARKET_RESOLUTION: &str = r#"
        INSERT INTO market_resolutions (
            market_id,
            status,
            proposed_winning_outcome,
            final_winning_outcome,
            payout_vector_hash,
            proposed_by_user_id,
            proposed_at,
            dispute_deadline,
            notes,
            disputed_by_user_id,
            disputed_at,
            dispute_reason,
            finalized_by_user_id,
            finalized_at,
            emergency_resolved_by_user_id,
            emergency_resolved_at
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8,
            $9, $10, $11, $12, $13, $14, $15, $16
        )
        ON CONFLICT (market_id) DO UPDATE
        SET
            status = EXCLUDED.status,
            proposed_winning_outcome = EXCLUDED.proposed_winning_outcome,
            final_winning_outcome = EXCLUDED.final_winning_outcome,
            payout_vector_hash = EXCLUDED.payout_vector_hash,
            proposed_by_user_id = EXCLUDED.proposed_by_user_id,
            proposed_at = EXCLUDED.proposed_at,
            dispute_deadline = EXCLUDED.dispute_deadline,
            notes = EXCLUDED.notes,
            disputed_by_user_id = EXCLUDED.disputed_by_user_id,
            disputed_at = EXCLUDED.disputed_at,
            dispute_reason = EXCLUDED.dispute_reason,
            finalized_by_user_id = EXCLUDED.finalized_by_user_id,
            finalized_at = EXCLUDED.finalized_at,
            emergency_resolved_by_user_id = EXCLUDED.emergency_resolved_by_user_id,
            emergency_resolved_at = EXCLUDED.emergency_resolved_at,
            updated_at = NOW()
        RETURNING
            market_id,
            status,
            proposed_winning_outcome,
            final_winning_outcome,
            payout_vector_hash,
            proposed_by_user_id,
            proposed_at,
            dispute_deadline,
            notes,
            disputed_by_user_id,
            disputed_at,
            dispute_reason,
            finalized_by_user_id,
            finalized_at,
            emergency_resolved_by_user_id,
            emergency_resolved_at,
            created_at,
            updated_at
    "#;

    pub const UPDATE_MARKET_RESOLUTION_DISPUTED: &str = r#"
        UPDATE market_resolutions
        SET
            status = 'disputed',
            disputed_by_user_id = $2,
            disputed_at = $3,
            dispute_reason = $4,
            updated_at = NOW()
        WHERE market_id = $1
        RETURNING
            market_id,
            status,
            proposed_winning_outcome,
            final_winning_outcome,
            payout_vector_hash,
            proposed_by_user_id,
            proposed_at,
            dispute_deadline,
            notes,
            disputed_by_user_id,
            disputed_at,
            dispute_reason,
            finalized_by_user_id,
            finalized_at,
            emergency_resolved_by_user_id,
            emergency_resolved_at,
            created_at,
            updated_at
    "#;

    pub const UPDATE_MARKET_RESOLUTION_FINALIZED: &str = r#"
        UPDATE market_resolutions
        SET
            status = 'finalized',
            final_winning_outcome = proposed_winning_outcome,
            finalized_by_user_id = $2,
            finalized_at = $3,
            updated_at = NOW()
        WHERE market_id = $1
        RETURNING
            market_id,
            status,
            proposed_winning_outcome,
            final_winning_outcome,
            payout_vector_hash,
            proposed_by_user_id,
            proposed_at,
            dispute_deadline,
            notes,
            disputed_by_user_id,
            disputed_at,
            dispute_reason,
            finalized_by_user_id,
            finalized_at,
            emergency_resolved_by_user_id,
            emergency_resolved_at,
            created_at,
            updated_at
    "#;

    pub const UPDATE_MARKET_EVENT_STANDALONE: &str = r#"
        UPDATE market_events
        SET
            title = $2,
            slug = $3,
            group_key = $4,
            series_key = $5,
            event_id = $6,
            group_id = $7,
            series_id = $8,
            oracle_address = $9,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
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
    "#;

    pub const INSERT_EVENT_NEG_RISK_CONFIG: &str = r#"
        INSERT INTO market_event_neg_risk_configs (
            event_id,
            registered,
            has_other,
            other_market_id,
            other_condition_id,
            registered_by_user_id,
            registered_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING
            event_id,
            registered,
            has_other,
            other_market_id,
            other_condition_id,
            registered_by_user_id,
            registered_at,
            created_at,
            updated_at
    "#;

    pub const LIST_PUBLISHED_MARKET_CONDITION_IDS: &str = r#"
        SELECT
            m.id AS market_id,
            m.condition_id
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND m.condition_id IS NOT NULL
    "#;

    pub const UPSERT_MARKET_PRICE_SNAPSHOT: &str = r#"
        INSERT INTO market_price_snapshots (
            market_id,
            condition_id,
            yes_bps,
            no_bps,
            synced_at
        )
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (market_id) DO UPDATE
        SET
            condition_id = EXCLUDED.condition_id,
            yes_bps = EXCLUDED.yes_bps,
            no_bps = EXCLUDED.no_bps,
            synced_at = NOW(),
            updated_at = NOW()
        RETURNING
            market_id,
            condition_id,
            yes_bps,
            no_bps,
            synced_at,
            created_at,
            updated_at
    "#;

    pub const GET_MARKET_PRICE_SNAPSHOT_BY_MARKET_ID: &str = r#"
        SELECT
            market_id,
            condition_id,
            yes_bps,
            no_bps,
            synced_at,
            created_at,
            updated_at
        FROM market_price_snapshots
        WHERE market_id = $1
    "#;

    pub const INSERT_MARKET_PRICE_HISTORY_SNAPSHOT: &str = r#"
        INSERT INTO market_price_history_snapshots (
            market_id,
            condition_id,
            yes_bps,
            no_bps,
            captured_at
        )
        VALUES ($1, $2, $3, $4, NOW())
        RETURNING
            id,
            market_id,
            condition_id,
            yes_bps,
            no_bps,
            captured_at,
            created_at
    "#;

    pub const LIST_MARKET_PRICE_HISTORY_SNAPSHOTS: &str = r#"
        SELECT
            id,
            market_id,
            condition_id,
            yes_bps,
            no_bps,
            captured_at,
            created_at
        FROM market_price_history_snapshots
        WHERE market_id = $1
        ORDER BY captured_at DESC, id DESC
        LIMIT $2
    "#;

    pub const GET_MARKET_TRADE_STATS_BY_MARKET_ID: &str = r#"
        SELECT
            market_id,
            volume_usd_cents,
            last_trade_yes_bps,
            last_trade_at,
            created_at,
            updated_at
        FROM market_trade_stats
        WHERE market_id = $1
    "#;

    pub const UPSERT_MARKET_TRADE_EXECUTION: &str = r#"
        INSERT INTO market_trade_stats (
            market_id,
            volume_usd_cents,
            last_trade_yes_bps,
            last_trade_at
        )
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (market_id) DO UPDATE
        SET
            volume_usd_cents = market_trade_stats.volume_usd_cents + EXCLUDED.volume_usd_cents,
            last_trade_yes_bps = EXCLUDED.last_trade_yes_bps,
            last_trade_at = EXCLUDED.last_trade_at,
            updated_at = NOW()
        RETURNING
            market_id,
            volume_usd_cents,
            last_trade_yes_bps,
            last_trade_at,
            created_at,
            updated_at
    "#;
}

pub struct PublicMarketListFilters<'a> {
    pub category_slug: Option<&'a str>,
    pub subcategory_slug: Option<&'a str>,
    pub tag_slug: Option<&'a str>,
    pub q: Option<&'a str>,
    pub featured: Option<bool>,
    pub breaking: Option<bool>,
    pub trading_status: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

pub struct PublicMarketSearchFilters<'a> {
    pub query: &'a str,
    pub tsquery: &'a str,
    pub category_slug: Option<&'a str>,
    pub subcategory_slug: Option<&'a str>,
    pub tag_slug: Option<&'a str>,
    pub featured: Option<bool>,
    pub breaking: Option<bool>,
    pub trading_status: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

pub struct PublicEventListFilters<'a> {
    pub public_only: bool,
    pub publication_status: Option<&'a str>,
    pub category_slug: Option<&'a str>,
    pub subcategory_slug: Option<&'a str>,
    pub tag_slug: Option<&'a str>,
    pub featured: Option<bool>,
    pub breaking: Option<bool>,
    pub limit: i64,
    pub offset: i64,
}

pub enum PublicMarketOrder {
    Featured,
    Newest,
}

pub async fn get_market_event_by_id(
    pool: &DbPool,
    event_id: Uuid,
) -> Result<Option<MarketEventRecord>, AuthError> {
    sqlx::query_as::<_, MarketEventRecord>(sql::GET_MARKET_EVENT_BY_ID)
        .bind(event_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_public_market_event_by_id(
    pool: &DbPool,
    event_id: Uuid,
) -> Result<Option<MarketEventRecord>, AuthError> {
    sqlx::query_as::<_, MarketEventRecord>(sql::GET_PUBLIC_MARKET_EVENT_BY_ID)
        .bind(event_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_public_market_by_id(
    pool: &DbPool,
    market_id: Uuid,
) -> Result<Option<MarketRecord>, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::GET_PUBLIC_MARKET_BY_ID)
        .bind(market_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_public_market_by_slug(
    pool: &DbPool,
    slug: &str,
) -> Result<Option<MarketRecord>, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::GET_PUBLIC_MARKET_BY_SLUG)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_public_market_by_condition_id(
    pool: &DbPool,
    condition_id: &str,
) -> Result<Option<MarketRecord>, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::GET_PUBLIC_MARKET_BY_CONDITION_ID)
        .bind(condition_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_public_markets_for_event(
    pool: &DbPool,
    event_id: Uuid,
) -> Result<Vec<MarketRecord>, AuthError> {
    list_public_markets_for_event_window(pool, event_id, None, None).await
}

pub async fn list_markets_for_event(
    pool: &DbPool,
    event_id: Uuid,
) -> Result<Vec<MarketRecord>, AuthError> {
    list_markets_for_event_filtered(pool, event_id, None, None, None).await
}

pub async fn list_public_markets_for_event_window(
    pool: &DbPool,
    event_id: Uuid,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<MarketRecord>, AuthError> {
    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            m.id,
            m.event_db_id,
            m.slug,
            m.label,
            m.question,
            m.question_id,
            m.condition_id,
            m.market_type,
            m.outcome_count,
            m.outcomes,
            m.end_time,
            m.sort_order,
            m.publication_status,
            m.trading_status,
            m.metadata_hash,
            m.oracle_address,
            m.created_at,
            m.updated_at
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.event_db_id = "#,
    );

    builder.push_bind(event_id);
    builder.push(
        r#"
            AND m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND e.visible = TRUE
        ORDER BY m.sort_order ASC, m.created_at ASC
        "#,
    );

    if let Some(limit) = limit {
        builder.push(" LIMIT ").push_bind(limit);
    }

    if let Some(offset) = offset {
        builder.push(" OFFSET ").push_bind(offset);
    }

    builder
        .build_query_as::<MarketRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_markets_for_event_filtered(
    pool: &DbPool,
    event_id: Uuid,
    publication_status: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<MarketRecord>, AuthError> {
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
        WHERE event_db_id = "#,
    );

    builder.push_bind(event_id);

    if let Some(publication_status) = publication_status {
        builder
            .push(" AND publication_status = ")
            .push_bind(publication_status);
    }

    builder.push(" ORDER BY sort_order ASC, created_at ASC");

    if let Some(limit) = limit {
        builder.push(" LIMIT ").push_bind(limit);
    }

    if let Some(offset) = offset {
        builder.push(" OFFSET ").push_bind(offset);
    }

    builder
        .build_query_as::<MarketRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn count_public_markets_for_event(
    pool: &DbPool,
    event_id: Uuid,
) -> Result<i64, AuthError> {
    sqlx::query_scalar::<_, i64>(sql::COUNT_PUBLIC_MARKETS_FOR_EVENT)
        .bind(event_id)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_published_market_condition_ids(
    pool: &DbPool,
) -> Result<Vec<MarketConditionRecord>, AuthError> {
    sqlx::query_as::<_, MarketConditionRecord>(sql::LIST_PUBLISHED_MARKET_CONDITION_IDS)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_market_price_snapshots_by_condition_ids(
    pool: &DbPool,
    condition_ids: &[String],
) -> Result<Vec<MarketPriceSnapshotRecord>, AuthError> {
    if condition_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            market_id,
            condition_id,
            yes_bps,
            no_bps,
            synced_at,
            created_at,
            updated_at
        FROM market_price_snapshots
        WHERE condition_id IN (
        "#,
    );

    {
        let mut separated = builder.separated(", ");
        for condition_id in condition_ids {
            separated.push_bind(condition_id);
        }
        separated.push_unseparated(")");
    }

    builder
        .build_query_as::<MarketPriceSnapshotRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_market_price_snapshot(
    pool: &DbPool,
    market_id: Uuid,
    condition_id: &str,
    yes_bps: i32,
    no_bps: i32,
) -> Result<MarketPriceSnapshotRecord, AuthError> {
    sqlx::query_as::<_, MarketPriceSnapshotRecord>(sql::UPSERT_MARKET_PRICE_SNAPSHOT)
        .bind(market_id)
        .bind(condition_id)
        .bind(yes_bps)
        .bind(no_bps)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_market_price_snapshot_by_market_id(
    pool: &DbPool,
    market_id: Uuid,
) -> Result<Option<MarketPriceSnapshotRecord>, AuthError> {
    sqlx::query_as::<_, MarketPriceSnapshotRecord>(sql::GET_MARKET_PRICE_SNAPSHOT_BY_MARKET_ID)
        .bind(market_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn insert_market_price_history_snapshot(
    pool: &DbPool,
    market_id: Uuid,
    condition_id: &str,
    yes_bps: i32,
    no_bps: i32,
) -> Result<MarketPriceHistorySnapshotRecord, AuthError> {
    sqlx::query_as::<_, MarketPriceHistorySnapshotRecord>(sql::INSERT_MARKET_PRICE_HISTORY_SNAPSHOT)
        .bind(market_id)
        .bind(condition_id)
        .bind(yes_bps)
        .bind(no_bps)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_market_price_history_snapshots(
    pool: &DbPool,
    market_id: Uuid,
    limit: i64,
) -> Result<Vec<MarketPriceHistorySnapshotRecord>, AuthError> {
    sqlx::query_as::<_, MarketPriceHistorySnapshotRecord>(sql::LIST_MARKET_PRICE_HISTORY_SNAPSHOTS)
        .bind(market_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_market_trade_stats_by_market_id(
    pool: &DbPool,
    market_id: Uuid,
) -> Result<Option<MarketTradeStatsRecord>, AuthError> {
    sqlx::query_as::<_, MarketTradeStatsRecord>(sql::GET_MARKET_TRADE_STATS_BY_MARKET_ID)
        .bind(market_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn upsert_market_trade_execution(
    pool: &DbPool,
    market_id: Uuid,
    volume_usd_cents: i64,
    last_trade_yes_bps: i32,
    last_trade_at: chrono::DateTime<chrono::Utc>,
) -> Result<MarketTradeStatsRecord, AuthError> {
    sqlx::query_as::<_, MarketTradeStatsRecord>(sql::UPSERT_MARKET_TRADE_EXECUTION)
        .bind(market_id)
        .bind(volume_usd_cents)
        .bind(last_trade_yes_bps)
        .bind(last_trade_at)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_market_trade_stats_by_market_ids(
    pool: &DbPool,
    market_ids: &[Uuid],
) -> Result<Vec<MarketTradeStatsRecord>, AuthError> {
    if market_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            market_id,
            volume_usd_cents,
            last_trade_yes_bps,
            last_trade_at,
            created_at,
            updated_at
        FROM market_trade_stats
        WHERE market_id IN (
        "#,
    );

    {
        let mut separated = builder.separated(", ");
        for market_id in market_ids {
            separated.push_bind(market_id);
        }
        separated.push_unseparated(")");
    }

    builder
        .build_query_as::<MarketTradeStatsRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_public_market_summaries(
    pool: &DbPool,
    filters: PublicMarketListFilters<'_>,
    order: PublicMarketOrder,
) -> Result<Vec<PublicMarketSummaryRecord>, AuthError> {
    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            m.id AS market_id,
            m.slug AS market_slug,
            m.label,
            m.question,
            m.question_id,
            m.condition_id,
            m.market_type,
            m.outcomes,
            m.end_time,
            m.sort_order,
            m.trading_status,
            m.created_at AS market_created_at,
            e.id AS event_id,
            e.slug AS event_slug,
            e.title AS event_title,
            e.category_slug,
            e.subcategory_slug,
            e.tag_slugs,
            e.image_url,
            e.summary_text,
            e.featured,
            e.breaking,
            e.neg_risk
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND e.visible = TRUE
        "#,
    );

    if let Some(category_slug) = filters.category_slug {
        builder
            .push(" AND e.category_slug = ")
            .push_bind(category_slug);
    }

    if let Some(subcategory_slug) = filters.subcategory_slug {
        builder
            .push(" AND e.subcategory_slug = ")
            .push_bind(subcategory_slug);
    }

    if let Some(tag_slug) = filters.tag_slug {
        builder
            .push(" AND ")
            .push_bind(tag_slug)
            .push(" = ANY(e.tag_slugs)");
    }

    if let Some(featured) = filters.featured {
        builder.push(" AND e.featured = ").push_bind(featured);
    }

    if let Some(breaking) = filters.breaking {
        builder.push(" AND e.breaking = ").push_bind(breaking);
    }

    if let Some(trading_status) = filters.trading_status {
        builder
            .push(" AND m.trading_status = ")
            .push_bind(trading_status);
    }

    if let Some(q) = filters.q {
        let search_pattern = format!("%{q}%");
        builder.push(" AND e.searchable = TRUE");
        builder.push(" AND (");
        builder
            .push("m.label ILIKE ")
            .push_bind(search_pattern.clone());
        builder
            .push(" OR m.question ILIKE ")
            .push_bind(search_pattern.clone());
        builder
            .push(" OR e.title ILIKE ")
            .push_bind(search_pattern.clone());
        builder.push(" OR e.slug ILIKE ").push_bind(search_pattern);
        builder.push(")");
    }

    match order {
        PublicMarketOrder::Featured => {
            builder.push(
                " ORDER BY e.featured DESC, e.breaking DESC, COALESCE(e.sort_at, m.created_at) DESC, m.sort_order ASC, m.created_at DESC",
            );
        }
        PublicMarketOrder::Newest => {
            builder.push(" ORDER BY m.created_at DESC, m.sort_order ASC");
        }
    }

    builder.push(" LIMIT ").push_bind(filters.limit);
    builder.push(" OFFSET ").push_bind(filters.offset);

    builder
        .build_query_as::<PublicMarketSummaryRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn search_public_market_summaries(
    pool: &DbPool,
    filters: PublicMarketSearchFilters<'_>,
) -> Result<Vec<PublicMarketSummaryRecord>, AuthError> {
    let exact_query = filters.query.to_lowercase();
    let escaped_query = escape_like_pattern(&exact_query);
    let prefix_query = format!("{escaped_query}%");
    let contains_query = format!("%{escaped_query}%");

    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            m.id AS market_id,
            m.slug AS market_slug,
            m.label,
            m.question,
            m.question_id,
            m.condition_id,
            m.market_type,
            m.outcomes,
            m.end_time,
            m.sort_order,
            m.trading_status,
            m.created_at AS market_created_at,
            e.id AS event_id,
            e.slug AS event_slug,
            e.title AS event_title,
            e.category_slug,
            e.subcategory_slug,
            e.tag_slugs,
            e.image_url,
            e.summary_text,
            e.featured,
            e.breaking,
            e.neg_risk
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND e.visible = TRUE
            AND e.searchable = TRUE
        "#,
    );

    if let Some(category_slug) = filters.category_slug {
        builder
            .push(" AND e.category_slug = ")
            .push_bind(category_slug);
    }

    if let Some(subcategory_slug) = filters.subcategory_slug {
        builder
            .push(" AND e.subcategory_slug = ")
            .push_bind(subcategory_slug);
    }

    if let Some(tag_slug) = filters.tag_slug {
        builder
            .push(" AND ")
            .push_bind(tag_slug)
            .push(" = ANY(e.tag_slugs)");
    }

    if let Some(featured) = filters.featured {
        builder.push(" AND e.featured = ").push_bind(featured);
    }

    if let Some(breaking) = filters.breaking {
        builder.push(" AND e.breaking = ").push_bind(breaking);
    }

    if let Some(trading_status) = filters.trading_status {
        builder
            .push(" AND m.trading_status = ")
            .push_bind(trading_status);
    }

    builder.push(
        " AND (to_tsvector('simple', coalesce(m.label, '') || ' ' || coalesce(m.question, '') || ' ' || coalesce(m.slug, '')) @@ to_tsquery('simple', ",
    );
    builder.push_bind(filters.tsquery);
    builder.push(
        ") OR to_tsvector('simple', coalesce(e.title, '') || ' ' || coalesce(e.slug, '')) @@ to_tsquery('simple', ",
    );
    builder.push_bind(filters.tsquery);
    builder.push("))");

    builder.push(" ORDER BY (CASE");
    builder
        .push(" WHEN lower(m.slug) = ")
        .push_bind(exact_query.clone());
    builder.push(" THEN 240");
    builder
        .push(" WHEN lower(e.slug) = ")
        .push_bind(exact_query.clone());
    builder.push(" THEN 220");
    builder
        .push(" WHEN lower(m.label) = ")
        .push_bind(exact_query.clone());
    builder.push(" THEN 200");
    builder
        .push(" WHEN lower(m.question) = ")
        .push_bind(exact_query.clone());
    builder.push(" THEN 180");
    builder
        .push(" WHEN lower(e.title) = ")
        .push_bind(exact_query.clone());
    builder.push(" THEN 160");
    builder
        .push(" WHEN lower(m.label) LIKE ")
        .push_bind(prefix_query.clone());
    builder.push(" ESCAPE '\\' THEN 120");
    builder
        .push(" WHEN lower(e.title) LIKE ")
        .push_bind(prefix_query.clone());
    builder.push(" ESCAPE '\\' THEN 110");
    builder
        .push(" WHEN lower(m.question) LIKE ")
        .push_bind(prefix_query.clone());
    builder.push(" ESCAPE '\\' THEN 100");
    builder
        .push(" WHEN lower(m.slug) LIKE ")
        .push_bind(prefix_query.clone());
    builder.push(" ESCAPE '\\' THEN 95");
    builder
        .push(" WHEN lower(e.slug) LIKE ")
        .push_bind(prefix_query);
    builder.push(" ESCAPE '\\' THEN 90");
    builder
        .push(" WHEN lower(m.label) LIKE ")
        .push_bind(contains_query.clone());
    builder.push(" ESCAPE '\\' THEN 60");
    builder
        .push(" WHEN lower(e.title) LIKE ")
        .push_bind(contains_query.clone());
    builder.push(" ESCAPE '\\' THEN 55");
    builder
        .push(" WHEN lower(m.question) LIKE ")
        .push_bind(contains_query);
    builder.push(" ESCAPE '\\' THEN 50");
    builder.push(" ELSE 0 END)");
    builder.push(
        " + (ts_rank_cd(to_tsvector('simple', coalesce(m.label, '') || ' ' || coalesce(m.question, '') || ' ' || coalesce(m.slug, '')), to_tsquery('simple', ",
    );
    builder.push_bind(filters.tsquery);
    builder.push(")) * 100)");
    builder.push(
        " + (ts_rank_cd(to_tsvector('simple', coalesce(e.title, '') || ' ' || coalesce(e.slug, '')), to_tsquery('simple', ",
    );
    builder.push_bind(filters.tsquery);
    builder.push(")) * 60) DESC");
    builder.push(" , e.featured DESC");
    builder.push(" , e.breaking DESC");
    builder.push(" , COALESCE(e.sort_at, m.created_at) DESC");
    builder.push(" , m.sort_order ASC");
    builder.push(" , m.created_at DESC");

    builder.push(" LIMIT ").push_bind(filters.limit);
    builder.push(" OFFSET ").push_bind(filters.offset);

    builder
        .build_query_as::<PublicMarketSummaryRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_public_market_summaries_for_event_ids(
    pool: &DbPool,
    event_ids: &[Uuid],
) -> Result<Vec<PublicMarketSummaryRecord>, AuthError> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            m.id AS market_id,
            m.slug AS market_slug,
            m.label,
            m.question,
            m.question_id,
            m.condition_id,
            m.market_type,
            m.outcomes,
            m.end_time,
            m.sort_order,
            m.trading_status,
            m.created_at AS market_created_at,
            e.id AS event_id,
            e.slug AS event_slug,
            e.title AS event_title,
            e.category_slug,
            e.subcategory_slug,
            e.tag_slugs,
            e.image_url,
            e.summary_text,
            e.featured,
            e.breaking,
            e.neg_risk
        FROM markets m
        INNER JOIN market_events e ON e.id = m.event_db_id
        WHERE
            m.publication_status = 'published'
            AND e.publication_status = 'published'
            AND e.visible = TRUE
            AND m.event_db_id IN (
        "#,
    );

    {
        let mut separated = builder.separated(", ");
        for event_id in event_ids {
            separated.push_bind(event_id);
        }
        separated.push_unseparated(")");
    }

    builder.push(" ORDER BY m.event_db_id ASC, m.sort_order ASC, m.created_at ASC");

    builder
        .build_query_as::<PublicMarketSummaryRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_public_event_summaries(
    pool: &DbPool,
    filters: PublicEventListFilters<'_>,
) -> Result<Vec<PublicEventSummaryRecord>, AuthError> {
    let mut builder = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            e.id AS event_id,
            e.slug AS event_slug,
            e.title AS event_title,
            e.category_slug,
            e.subcategory_slug,
            e.tag_slugs,
            e.image_url,
            e.summary_text,
            e.featured,
            e.breaking,
            e.neg_risk,
            e.publication_status,
            e.starts_at,
            e.sort_at,
            e.created_at,
            COUNT(m.id)::BIGINT AS market_count
        FROM market_events e
        LEFT JOIN markets m
            ON m.event_db_id = e.id
            AND m.publication_status = 'published'
        WHERE
            1 = 1
        "#,
    );

    if let Some(publication_status) = filters.publication_status {
        builder
            .push(" AND e.publication_status = ")
            .push_bind(publication_status);
    } else if filters.public_only {
        builder.push(" AND e.publication_status = 'published'");
        builder.push(" AND e.visible = TRUE");
    }

    if let Some(category_slug) = filters.category_slug {
        builder
            .push(" AND e.category_slug = ")
            .push_bind(category_slug);
    }

    if let Some(subcategory_slug) = filters.subcategory_slug {
        builder
            .push(" AND e.subcategory_slug = ")
            .push_bind(subcategory_slug);
    }

    if let Some(tag_slug) = filters.tag_slug {
        builder
            .push(" AND ")
            .push_bind(tag_slug)
            .push(" = ANY(e.tag_slugs)");
    }

    if let Some(featured) = filters.featured {
        builder.push(" AND e.featured = ").push_bind(featured);
    }

    if let Some(breaking) = filters.breaking {
        builder.push(" AND e.breaking = ").push_bind(breaking);
    }

    builder.push(
        " GROUP BY e.id ORDER BY e.featured DESC, e.breaking DESC, COALESCE(e.sort_at, e.starts_at, e.created_at) DESC, e.created_at DESC",
    );
    builder.push(" LIMIT ").push_bind(filters.limit);
    builder.push(" OFFSET ").push_bind(filters.offset);

    builder
        .build_query_as::<PublicEventSummaryRecord>()
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_category_summaries(
    pool: &DbPool,
) -> Result<Vec<CategorySummaryRecord>, AuthError> {
    sqlx::query_as::<_, CategorySummaryRecord>(sql::LIST_CATEGORY_SUMMARIES)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_category_summary_by_slug(
    pool: &DbPool,
    slug: &str,
) -> Result<Option<CategorySummaryRecord>, AuthError> {
    sqlx::query_as::<_, CategorySummaryRecord>(sql::GET_CATEGORY_SUMMARY_BY_SLUG)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn list_tag_summaries(pool: &DbPool) -> Result<Vec<TagSummaryRecord>, AuthError> {
    sqlx::query_as::<_, TagSummaryRecord>(sql::LIST_TAG_SUMMARIES)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn create_market_event(
    pool: &DbPool,
    event: &NewMarketEventRecord,
) -> Result<MarketEventRecord, AuthError> {
    sqlx::query_as::<_, MarketEventRecord>(sql::INSERT_MARKET_EVENT)
        .bind(event.id)
        .bind(&event.title)
        .bind(&event.slug)
        .bind(&event.category_slug)
        .bind(event.subcategory_slug.as_deref())
        .bind(&event.tag_slugs)
        .bind(event.image_url.as_deref())
        .bind(event.summary_text.as_deref())
        .bind(&event.rules_text)
        .bind(event.context_text.as_deref())
        .bind(event.additional_context.as_deref())
        .bind(&event.resolution_sources)
        .bind(&event.resolution_timezone)
        .bind(event.starts_at)
        .bind(event.sort_at)
        .bind(event.featured)
        .bind(event.breaking)
        .bind(event.searchable)
        .bind(event.visible)
        .bind(event.hide_resolved_by_default)
        .bind(&event.group_key)
        .bind(&event.series_key)
        .bind(&event.event_id)
        .bind(&event.group_id)
        .bind(&event.series_id)
        .bind(event.neg_risk)
        .bind(event.oracle_address.as_deref())
        .bind(&event.publication_status)
        .bind(event.published_tx_hash.as_deref())
        .bind(event.created_by_user_id)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn create_market_bundle(
    pool: &DbPool,
    event: &NewMarketEventRecord,
    market: &NewMarketRecord,
) -> Result<(MarketEventRecord, MarketRecord), AuthError> {
    let mut tx = pool.begin().await?;

    let event_record = sqlx::query_as::<_, MarketEventRecord>(sql::INSERT_MARKET_EVENT)
        .bind(event.id)
        .bind(&event.title)
        .bind(&event.slug)
        .bind(&event.category_slug)
        .bind(event.subcategory_slug.as_deref())
        .bind(&event.tag_slugs)
        .bind(event.image_url.as_deref())
        .bind(event.summary_text.as_deref())
        .bind(&event.rules_text)
        .bind(event.context_text.as_deref())
        .bind(event.additional_context.as_deref())
        .bind(&event.resolution_sources)
        .bind(&event.resolution_timezone)
        .bind(event.starts_at)
        .bind(event.sort_at)
        .bind(event.featured)
        .bind(event.breaking)
        .bind(event.searchable)
        .bind(event.visible)
        .bind(event.hide_resolved_by_default)
        .bind(&event.group_key)
        .bind(&event.series_key)
        .bind(&event.event_id)
        .bind(&event.group_id)
        .bind(&event.series_id)
        .bind(event.neg_risk)
        .bind(event.oracle_address.as_deref())
        .bind(&event.publication_status)
        .bind(event.published_tx_hash.as_deref())
        .bind(event.created_by_user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

    let market_record = sqlx::query_as::<_, MarketRecord>(sql::INSERT_MARKET)
        .bind(market.id)
        .bind(market.event_db_id)
        .bind(&market.slug)
        .bind(&market.label)
        .bind(&market.question)
        .bind(&market.question_id)
        .bind(market.condition_id.as_deref())
        .bind(&market.market_type)
        .bind(market.outcome_count)
        .bind(&market.outcomes)
        .bind(market.end_time)
        .bind(market.sort_order)
        .bind(&market.publication_status)
        .bind(&market.trading_status)
        .bind(market.metadata_hash.as_deref())
        .bind(&market.oracle_address)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

    tx.commit().await?;

    Ok((event_record, market_record))
}

pub async fn create_market_records(
    pool: &DbPool,
    markets: &[NewMarketRecord],
) -> Result<Vec<MarketRecord>, AuthError> {
    let mut tx = pool.begin().await?;
    let mut market_records = Vec::with_capacity(markets.len());

    for market in markets {
        let market_record = sqlx::query_as::<_, MarketRecord>(sql::INSERT_MARKET)
            .bind(market.id)
            .bind(market.event_db_id)
            .bind(&market.slug)
            .bind(&market.label)
            .bind(&market.question)
            .bind(&market.question_id)
            .bind(market.condition_id.as_deref())
            .bind(&market.market_type)
            .bind(market.outcome_count)
            .bind(&market.outcomes)
            .bind(market.end_time)
            .bind(market.sort_order)
            .bind(&market.publication_status)
            .bind(&market.trading_status)
            .bind(market.metadata_hash.as_deref())
            .bind(&market.oracle_address)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_insert_error)?;

        market_records.push(market_record);
    }

    tx.commit().await?;

    Ok(market_records)
}

pub async fn create_market_record(
    pool: &DbPool,
    market: &NewMarketRecord,
) -> Result<MarketRecord, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::INSERT_MARKET)
        .bind(market.id)
        .bind(market.event_db_id)
        .bind(&market.slug)
        .bind(&market.label)
        .bind(&market.question)
        .bind(&market.question_id)
        .bind(market.condition_id.as_deref())
        .bind(&market.market_type)
        .bind(market.outcome_count)
        .bind(&market.outcomes)
        .bind(market.end_time)
        .bind(market.sort_order)
        .bind(&market.publication_status)
        .bind(&market.trading_status)
        .bind(market.metadata_hash.as_deref())
        .bind(&market.oracle_address)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn get_market_by_id(
    pool: &DbPool,
    market_id: Uuid,
) -> Result<Option<MarketRecord>, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::GET_MARKET_BY_ID)
        .bind(market_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_market_by_slug(
    pool: &DbPool,
    slug: &str,
) -> Result<Option<MarketRecord>, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::GET_MARKET_BY_SLUG)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_market_resolution_by_market_id(
    pool: &DbPool,
    market_id: Uuid,
) -> Result<Option<MarketResolutionRecord>, AuthError> {
    sqlx::query_as::<_, MarketResolutionRecord>(sql::GET_MARKET_RESOLUTION_BY_MARKET_ID)
        .bind(market_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_market_auto_resolution_config_by_market_id(
    pool: &DbPool,
    market_id: Uuid,
) -> Result<Option<MarketAutoResolutionConfigRecord>, AuthError> {
    sqlx::query_as::<_, MarketAutoResolutionConfigRecord>(
        sql::GET_MARKET_AUTO_RESOLUTION_CONFIG_BY_MARKET_ID,
    )
    .bind(market_id)
    .fetch_optional(pool)
    .await
    .map_err(AuthError::from)
}

pub async fn upsert_market_auto_resolution_config(
    pool: &DbPool,
    config: &NewMarketAutoResolutionConfigRecord,
) -> Result<MarketAutoResolutionConfigRecord, AuthError> {
    sqlx::query_as::<_, MarketAutoResolutionConfigRecord>(sql::UPSERT_MARKET_AUTO_RESOLUTION_CONFIG)
        .bind(config.market_id)
        .bind(&config.provider)
        .bind(&config.product_id)
        .bind(config.start_time)
        .bind(config.start_price.as_deref())
        .bind(config.start_price_captured_at)
        .bind(config.end_price.as_deref())
        .bind(config.end_price_captured_at)
        .bind(config.up_outcome_index)
        .bind(config.down_outcome_index)
        .bind(config.tie_outcome_index)
        .bind(config.last_error.as_deref())
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn update_market_auto_resolution_start_price(
    pool: &DbPool,
    market_id: Uuid,
    price: &str,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Result<MarketAutoResolutionConfigRecord, AuthError> {
    sqlx::query_as::<_, MarketAutoResolutionConfigRecord>(
        sql::UPDATE_MARKET_AUTO_RESOLUTION_START_PRICE,
    )
    .bind(market_id)
    .bind(price)
    .bind(captured_at)
    .fetch_one(pool)
    .await
    .map_err(map_insert_error)
}

pub async fn update_market_auto_resolution_end_price(
    pool: &DbPool,
    market_id: Uuid,
    price: &str,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Result<MarketAutoResolutionConfigRecord, AuthError> {
    sqlx::query_as::<_, MarketAutoResolutionConfigRecord>(
        sql::UPDATE_MARKET_AUTO_RESOLUTION_END_PRICE,
    )
    .bind(market_id)
    .bind(price)
    .bind(captured_at)
    .fetch_one(pool)
    .await
    .map_err(map_insert_error)
}

pub async fn update_market_auto_resolution_error(
    pool: &DbPool,
    market_id: Uuid,
    last_error: Option<&str>,
) -> Result<MarketAutoResolutionConfigRecord, AuthError> {
    sqlx::query_as::<_, MarketAutoResolutionConfigRecord>(sql::UPDATE_MARKET_AUTO_RESOLUTION_ERROR)
        .bind(market_id)
        .bind(last_error)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn list_pending_market_auto_resolution_records(
    pool: &DbPool,
) -> Result<Vec<PendingMarketAutoResolutionRecord>, AuthError> {
    sqlx::query_as::<_, PendingMarketAutoResolutionRecord>(
        sql::LIST_PENDING_MARKET_AUTO_RESOLUTION_RECORDS,
    )
    .fetch_all(pool)
    .await
    .map_err(AuthError::from)
}

pub async fn upsert_market_auto_create_series(
    pool: &DbPool,
    record: &NewMarketAutoCreateSeriesRecord,
) -> Result<MarketAutoCreateSeriesRecord, AuthError> {
    sqlx::query_as::<_, MarketAutoCreateSeriesRecord>(sql::UPSERT_MARKET_AUTO_CREATE_SERIES)
        .bind(record.id)
        .bind(&record.provider)
        .bind(&record.product_id)
        .bind(&record.title_prefix)
        .bind(&record.slug_prefix)
        .bind(&record.category_slug)
        .bind(record.subcategory_slug.as_deref())
        .bind(&record.tag_slugs)
        .bind(record.image_url.as_deref())
        .bind(record.summary_text.as_deref())
        .bind(&record.rules_text)
        .bind(record.context_text.as_deref())
        .bind(record.additional_context.as_deref())
        .bind(&record.resolution_sources)
        .bind(&record.resolution_timezone)
        .bind(record.start_time)
        .bind(record.cadence_seconds)
        .bind(record.market_duration_seconds)
        .bind(&record.oracle_address)
        .bind(&record.outcomes)
        .bind(record.up_outcome_index)
        .bind(record.down_outcome_index)
        .bind(record.tie_outcome_index)
        .bind(record.featured)
        .bind(record.breaking)
        .bind(record.searchable)
        .bind(record.visible)
        .bind(record.hide_resolved_by_default)
        .bind(record.active)
        .bind(record.last_created_slot_start)
        .bind(record.created_by_user_id)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn list_active_market_auto_create_series(
    pool: &DbPool,
) -> Result<Vec<MarketAutoCreateSeriesRecord>, AuthError> {
    sqlx::query_as::<_, MarketAutoCreateSeriesRecord>(sql::LIST_ACTIVE_MARKET_AUTO_CREATE_SERIES)
        .fetch_all(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn update_market_auto_create_series_last_created_slot_start(
    pool: &DbPool,
    series_id: Uuid,
    last_created_slot_start: chrono::DateTime<chrono::Utc>,
) -> Result<MarketAutoCreateSeriesRecord, AuthError> {
    sqlx::query_as::<_, MarketAutoCreateSeriesRecord>(
        sql::UPDATE_MARKET_AUTO_CREATE_SERIES_LAST_CREATED_SLOT_START,
    )
    .bind(series_id)
    .bind(last_created_slot_start)
    .fetch_one(pool)
    .await
    .map_err(map_insert_error)
}

pub async fn get_event_neg_risk_config_by_event_id(
    pool: &DbPool,
    event_id: Uuid,
) -> Result<Option<MarketEventNegRiskConfigRecord>, AuthError> {
    sqlx::query_as::<_, MarketEventNegRiskConfigRecord>(sql::GET_EVENT_NEG_RISK_CONFIG_BY_EVENT_ID)
        .bind(event_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn count_markets_for_event(pool: &DbPool, event_id: Uuid) -> Result<i64, AuthError> {
    sqlx::query_scalar::<_, i64>(sql::COUNT_MARKETS_FOR_EVENT)
        .bind(event_id)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn update_market(
    pool: &DbPool,
    market: &MarketRecord,
) -> Result<MarketRecord, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::UPDATE_MARKET)
        .bind(market.id)
        .bind(&market.slug)
        .bind(&market.label)
        .bind(&market.question)
        .bind(&market.question_id)
        .bind(market.outcome_count)
        .bind(&market.outcomes)
        .bind(market.end_time)
        .bind(market.sort_order)
        .bind(&market.oracle_address)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn update_market_trading_status(
    pool: &DbPool,
    market_id: Uuid,
    trading_status: &str,
) -> Result<MarketRecord, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::UPDATE_MARKET_TRADING_STATUS)
        .bind(market_id)
        .bind(trading_status)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn update_market_event_publication_status(
    pool: &DbPool,
    event_id: Uuid,
    publication_status: &str,
    published_tx_hash: Option<&str>,
) -> Result<MarketEventRecord, AuthError> {
    sqlx::query_as::<_, MarketEventRecord>(sql::UPDATE_MARKET_EVENT_PUBLICATION)
        .bind(event_id)
        .bind(publication_status)
        .bind(published_tx_hash)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn update_market_publication_status(
    pool: &DbPool,
    market_id: Uuid,
    publication_status: &str,
    condition_id: Option<&str>,
) -> Result<MarketRecord, AuthError> {
    sqlx::query_as::<_, MarketRecord>(sql::UPDATE_MARKET_PUBLICATION)
        .bind(market_id)
        .bind(publication_status)
        .bind(condition_id)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn upsert_market_resolution_with_trading_status(
    pool: &DbPool,
    resolution: &NewMarketResolutionRecord,
    trading_status: &str,
) -> Result<(MarketRecord, MarketResolutionRecord), AuthError> {
    let mut tx = pool.begin().await?;

    let market = sqlx::query_as::<_, MarketRecord>(sql::UPDATE_MARKET_TRADING_STATUS)
        .bind(resolution.market_id)
        .bind(trading_status)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

    let resolution = sqlx::query_as::<_, MarketResolutionRecord>(sql::UPSERT_MARKET_RESOLUTION)
        .bind(resolution.market_id)
        .bind(&resolution.status)
        .bind(resolution.proposed_winning_outcome)
        .bind(resolution.final_winning_outcome)
        .bind(&resolution.payout_vector_hash)
        .bind(resolution.proposed_by_user_id)
        .bind(resolution.proposed_at)
        .bind(resolution.dispute_deadline)
        .bind(resolution.notes.as_deref())
        .bind(resolution.disputed_by_user_id)
        .bind(resolution.disputed_at)
        .bind(resolution.dispute_reason.as_deref())
        .bind(resolution.finalized_by_user_id)
        .bind(resolution.finalized_at)
        .bind(resolution.emergency_resolved_by_user_id)
        .bind(resolution.emergency_resolved_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

    tx.commit().await?;

    Ok((market, resolution))
}

pub async fn dispute_market_resolution(
    pool: &DbPool,
    market_id: Uuid,
    disputed_by_user_id: Uuid,
    disputed_at: chrono::DateTime<chrono::Utc>,
    dispute_reason: &str,
) -> Result<(MarketRecord, MarketResolutionRecord), AuthError> {
    let mut tx = pool.begin().await?;

    let market = sqlx::query_as::<_, MarketRecord>(sql::UPDATE_MARKET_TRADING_STATUS)
        .bind(market_id)
        .bind("paused")
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

    let resolution =
        sqlx::query_as::<_, MarketResolutionRecord>(sql::UPDATE_MARKET_RESOLUTION_DISPUTED)
            .bind(market_id)
            .bind(disputed_by_user_id)
            .bind(disputed_at)
            .bind(dispute_reason)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_insert_error)?;

    tx.commit().await?;

    Ok((market, resolution))
}

pub async fn finalize_market_resolution(
    pool: &DbPool,
    market_id: Uuid,
    finalized_by_user_id: Uuid,
    finalized_at: chrono::DateTime<chrono::Utc>,
) -> Result<(MarketRecord, MarketResolutionRecord), AuthError> {
    let mut tx = pool.begin().await?;

    let market = sqlx::query_as::<_, MarketRecord>(sql::UPDATE_MARKET_TRADING_STATUS)
        .bind(market_id)
        .bind("resolved")
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

    let resolution =
        sqlx::query_as::<_, MarketResolutionRecord>(sql::UPDATE_MARKET_RESOLUTION_FINALIZED)
            .bind(market_id)
            .bind(finalized_by_user_id)
            .bind(finalized_at)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_insert_error)?;

    tx.commit().await?;

    Ok((market, resolution))
}

pub async fn update_market_event_for_standalone(
    pool: &DbPool,
    event: &MarketEventRecord,
    title: &str,
    slug: &str,
    group_key: &str,
    series_key: &str,
    event_id: &str,
    group_id: &str,
    series_id: &str,
    oracle_address: &str,
) -> Result<MarketEventRecord, AuthError> {
    sqlx::query_as::<_, MarketEventRecord>(sql::UPDATE_MARKET_EVENT_STANDALONE)
        .bind(event.id)
        .bind(title)
        .bind(slug)
        .bind(group_key)
        .bind(series_key)
        .bind(event_id)
        .bind(group_id)
        .bind(series_id)
        .bind(oracle_address)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

pub async fn create_event_neg_risk_config(
    pool: &DbPool,
    config: &NewMarketEventNegRiskConfigRecord,
) -> Result<MarketEventNegRiskConfigRecord, AuthError> {
    sqlx::query_as::<_, MarketEventNegRiskConfigRecord>(sql::INSERT_EVENT_NEG_RISK_CONFIG)
        .bind(config.event_id)
        .bind(config.registered)
        .bind(config.has_other)
        .bind(config.other_market_id)
        .bind(config.other_condition_id.as_deref())
        .bind(config.registered_by_user_id)
        .bind(config.registered_at)
        .fetch_one(pool)
        .await
        .map_err(map_insert_error)
}

fn escape_like_pattern(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn map_insert_error(error: sqlx::Error) -> AuthError {
    match unique_constraint(&error) {
        Some("market_events_slug_key") => AuthError::conflict("event slug already exists"),
        Some("market_events_event_id_key") => AuthError::conflict("event already exists"),
        Some("market_auto_create_series_slug_prefix_key") => {
            AuthError::conflict("market series slug_prefix already exists")
        }
        Some("market_event_neg_risk_configs_pkey") => {
            AuthError::conflict("neg risk event already registered")
        }
        Some("markets_slug_key") => AuthError::conflict("market slug already exists"),
        Some("markets_question_id_key") => AuthError::conflict("market question already exists"),
        Some("markets_condition_id_key") => AuthError::conflict("market condition already exists"),
        Some("markets_event_db_id_sort_order_key") => {
            AuthError::conflict("market sort_order already exists for this event")
        }
        _ => AuthError::from(error),
    }
}

fn unique_constraint(error: &sqlx::Error) -> Option<&str> {
    match error {
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505") =>
        {
            database_error.constraint()
        }
        _ => None,
    }
}
