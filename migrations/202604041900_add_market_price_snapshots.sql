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
