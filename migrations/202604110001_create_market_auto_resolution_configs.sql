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
