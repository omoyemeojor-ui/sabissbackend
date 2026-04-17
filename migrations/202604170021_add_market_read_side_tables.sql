CREATE TABLE IF NOT EXISTS market_resolutions (
    market_id UUID PRIMARY KEY REFERENCES markets(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    proposed_winning_outcome INTEGER NOT NULL,
    final_winning_outcome INTEGER,
    payout_vector_hash TEXT NOT NULL,
    proposed_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    proposed_at TIMESTAMPTZ NOT NULL,
    dispute_deadline TIMESTAMPTZ NOT NULL,
    notes TEXT,
    disputed_by_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    disputed_at TIMESTAMPTZ,
    dispute_reason TEXT,
    finalized_by_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    finalized_at TIMESTAMPTZ,
    emergency_resolved_by_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    emergency_resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_resolutions_status_check CHECK (
        status IN ('proposed', 'disputed', 'finalized', 'emergency_resolved')
    )
);

CREATE TABLE IF NOT EXISTS market_event_neg_risk_configs (
    event_id UUID PRIMARY KEY REFERENCES market_events(id) ON DELETE CASCADE,
    registered BOOLEAN NOT NULL DEFAULT TRUE,
    has_other BOOLEAN NOT NULL DEFAULT FALSE,
    other_market_id UUID REFERENCES markets(id) ON DELETE SET NULL,
    other_condition_id TEXT,
    registered_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    registered_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS market_price_snapshots (
    market_id UUID PRIMARY KEY REFERENCES markets(id) ON DELETE CASCADE,
    condition_id TEXT NOT NULL UNIQUE,
    yes_bps INTEGER NOT NULL,
    no_bps INTEGER NOT NULL,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_price_snapshots_yes_bps_check CHECK (yes_bps >= 0 AND yes_bps <= 10000),
    CONSTRAINT market_price_snapshots_no_bps_check CHECK (no_bps >= 0 AND no_bps <= 10000),
    CONSTRAINT market_price_snapshots_sum_check CHECK (yes_bps + no_bps = 10000)
);

CREATE INDEX IF NOT EXISTS market_price_snapshots_synced_at_idx
ON market_price_snapshots (synced_at DESC);

CREATE TABLE IF NOT EXISTS market_trade_stats (
    market_id UUID PRIMARY KEY REFERENCES markets(id) ON DELETE CASCADE,
    volume_usd_cents BIGINT NOT NULL DEFAULT 0,
    last_trade_yes_bps INTEGER,
    last_trade_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_trade_stats_volume_usd_cents_check CHECK (volume_usd_cents >= 0),
    CONSTRAINT market_trade_stats_last_trade_yes_bps_check CHECK (
        last_trade_yes_bps IS NULL OR (last_trade_yes_bps >= 0 AND last_trade_yes_bps <= 10000)
    )
);

CREATE INDEX IF NOT EXISTS market_trade_stats_updated_at_idx
ON market_trade_stats (updated_at DESC);

CREATE TABLE IF NOT EXISTS market_price_history_snapshots (
    id BIGSERIAL PRIMARY KEY,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    condition_id TEXT NOT NULL,
    yes_bps INTEGER NOT NULL,
    no_bps INTEGER NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_price_history_snapshots_yes_bps_check CHECK (yes_bps >= 0 AND yes_bps <= 10000),
    CONSTRAINT market_price_history_snapshots_no_bps_check CHECK (no_bps >= 0 AND no_bps <= 10000),
    CONSTRAINT market_price_history_snapshots_sum_check CHECK (yes_bps + no_bps = 10000)
);

CREATE INDEX IF NOT EXISTS market_price_history_snapshots_market_captured_idx
ON market_price_history_snapshots (market_id, captured_at DESC);

CREATE INDEX IF NOT EXISTS market_price_history_snapshots_condition_captured_idx
ON market_price_history_snapshots (condition_id, captured_at DESC);

CREATE TABLE IF NOT EXISTS market_auto_resolution_configs (
    market_id UUID PRIMARY KEY REFERENCES markets(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    product_id TEXT NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    start_price TEXT,
    start_price_captured_at TIMESTAMPTZ,
    end_price TEXT,
    end_price_captured_at TIMESTAMPTZ,
    up_outcome_index INTEGER NOT NULL DEFAULT 0,
    down_outcome_index INTEGER NOT NULL DEFAULT 1,
    tie_outcome_index INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_auto_resolution_configs_provider_check
        CHECK (provider IN ('coinbase')),
    CONSTRAINT market_auto_resolution_configs_distinct_outcomes_check
        CHECK (up_outcome_index <> down_outcome_index)
);

CREATE INDEX IF NOT EXISTS market_auto_resolution_configs_provider_start_time_idx
ON market_auto_resolution_configs (provider, start_time);

CREATE INDEX IF NOT EXISTS market_auto_resolution_configs_provider_end_time_idx
ON market_auto_resolution_configs (provider, end_price_captured_at, start_price_captured_at);

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
