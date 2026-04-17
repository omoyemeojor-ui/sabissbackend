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
