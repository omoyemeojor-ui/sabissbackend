CREATE TABLE IF NOT EXISTS market_order_fills (
    id UUID PRIMARY KEY,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES market_events(id) ON DELETE CASCADE,
    condition_id TEXT NOT NULL,
    match_type TEXT NOT NULL,
    buy_order_id UUID REFERENCES market_orders(id) ON DELETE SET NULL,
    sell_order_id UUID REFERENCES market_orders(id) ON DELETE SET NULL,
    yes_order_id UUID REFERENCES market_orders(id) ON DELETE SET NULL,
    no_order_id UUID REFERENCES market_orders(id) ON DELETE SET NULL,
    outcome_index INTEGER,
    fill_amount TEXT NOT NULL,
    collateral_amount TEXT NOT NULL,
    yes_price_bps INTEGER NOT NULL,
    no_price_bps INTEGER NOT NULL,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_order_fills_match_type_check CHECK (
        match_type IN ('direct', 'complementary_buy', 'complementary_sell')
    ),
    CONSTRAINT market_order_fills_outcome_index_check CHECK (
        outcome_index IS NULL OR outcome_index IN (0, 1)
    ),
    CONSTRAINT market_order_fills_yes_price_bps_check CHECK (
        yes_price_bps >= 0 AND yes_price_bps <= 10000
    ),
    CONSTRAINT market_order_fills_no_price_bps_check CHECK (
        no_price_bps >= 0 AND no_price_bps <= 10000
    ),
    CONSTRAINT market_order_fills_price_sum_check CHECK (
        yes_price_bps + no_price_bps = 10000
    ),
    CONSTRAINT market_order_fills_condition_id_nonempty_check CHECK (length(condition_id) > 0),
    CONSTRAINT market_order_fills_fill_amount_nonempty_check CHECK (length(fill_amount) > 0),
    CONSTRAINT market_order_fills_collateral_amount_nonempty_check CHECK (length(collateral_amount) > 0),
    CONSTRAINT market_order_fills_tx_hash_nonempty_check CHECK (length(tx_hash) > 0)
);

CREATE INDEX IF NOT EXISTS market_order_fills_market_created_idx
ON market_order_fills (market_id, created_at DESC);

CREATE INDEX IF NOT EXISTS market_order_fills_buy_order_idx
ON market_order_fills (buy_order_id)
WHERE buy_order_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS market_order_fills_sell_order_idx
ON market_order_fills (sell_order_id)
WHERE sell_order_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS market_order_fills_yes_order_idx
ON market_order_fills (yes_order_id)
WHERE yes_order_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS market_order_fills_no_order_idx
ON market_order_fills (no_order_id)
WHERE no_order_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS market_comments (
    id UUID PRIMARY KEY,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES market_events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_comments_body_nonempty_check CHECK (length(trim(body)) > 0)
);

CREATE INDEX IF NOT EXISTS market_comments_market_created_idx
ON market_comments (market_id, created_at DESC);

CREATE INDEX IF NOT EXISTS market_comments_user_created_idx
ON market_comments (user_id, created_at DESC);
