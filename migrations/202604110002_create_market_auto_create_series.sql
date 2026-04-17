CREATE TABLE IF NOT EXISTS market_auto_create_series (
    id UUID PRIMARY KEY,
    provider TEXT NOT NULL,
    product_id TEXT NOT NULL,
    title_prefix TEXT NOT NULL,
    slug_prefix TEXT NOT NULL UNIQUE,
    category_slug TEXT NOT NULL,
    subcategory_slug TEXT,
    tag_slugs TEXT[] NOT NULL DEFAULT '{}',
    image_url TEXT,
    summary_text TEXT,
    rules_text TEXT NOT NULL,
    context_text TEXT,
    additional_context TEXT,
    resolution_sources TEXT[] NOT NULL DEFAULT '{}',
    resolution_timezone TEXT NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    cadence_seconds INTEGER NOT NULL,
    market_duration_seconds INTEGER NOT NULL,
    oracle_address TEXT NOT NULL,
    outcomes TEXT[] NOT NULL,
    up_outcome_index INTEGER NOT NULL DEFAULT 0,
    down_outcome_index INTEGER NOT NULL DEFAULT 1,
    tie_outcome_index INTEGER NOT NULL DEFAULT 0,
    featured BOOLEAN NOT NULL DEFAULT FALSE,
    breaking BOOLEAN NOT NULL DEFAULT FALSE,
    searchable BOOLEAN NOT NULL DEFAULT TRUE,
    visible BOOLEAN NOT NULL DEFAULT TRUE,
    hide_resolved_by_default BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    last_created_slot_start TIMESTAMPTZ,
    created_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_auto_create_series_provider_check
        CHECK (provider IN ('coinbase')),
    CONSTRAINT market_auto_create_series_cadence_seconds_check
        CHECK (cadence_seconds > 0),
    CONSTRAINT market_auto_create_series_market_duration_seconds_check
        CHECK (market_duration_seconds > 0),
    CONSTRAINT market_auto_create_series_outcomes_length_check
        CHECK (array_length(outcomes, 1) = 2),
    CONSTRAINT market_auto_create_series_distinct_outcomes_check
        CHECK (outcomes[1] <> outcomes[2]),
    CONSTRAINT market_auto_create_series_distinct_indexes_check
        CHECK (up_outcome_index <> down_outcome_index)
);

CREATE INDEX IF NOT EXISTS market_auto_create_series_active_start_time_idx
ON market_auto_create_series (active, start_time);
