CREATE TABLE IF NOT EXISTS market_orders (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES market_events(id) ON DELETE CASCADE,
    wallet_address TEXT NOT NULL,
    account_kind TEXT NOT NULL,
    condition_id TEXT NOT NULL,
    outcome_index INTEGER NOT NULL,
    side TEXT NOT NULL,
    price_bps INTEGER NOT NULL,
    amount TEXT NOT NULL,
    filled_amount TEXT NOT NULL DEFAULT '0',
    remaining_amount TEXT NOT NULL,
    expiry_epoch_seconds BIGINT,
    salt TEXT NOT NULL,
    signature TEXT NOT NULL,
    order_hash TEXT NOT NULL UNIQUE,
    order_digest TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'open',
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT market_orders_outcome_index_check CHECK (outcome_index IN (0, 1)),
    CONSTRAINT market_orders_side_check CHECK (side IN ('buy', 'sell')),
    CONSTRAINT market_orders_price_bps_check CHECK (price_bps >= 0 AND price_bps <= 10000),
    CONSTRAINT market_orders_status_check CHECK (
        status IN ('open', 'partially_filled', 'filled', 'cancelled')
    ),
    CONSTRAINT market_orders_amount_nonempty_check CHECK (length(amount) > 0),
    CONSTRAINT market_orders_filled_amount_nonempty_check CHECK (length(filled_amount) > 0),
    CONSTRAINT market_orders_remaining_amount_nonempty_check CHECK (length(remaining_amount) > 0),
    CONSTRAINT market_orders_salt_nonempty_check CHECK (length(salt) > 0),
    CONSTRAINT market_orders_signature_nonempty_check CHECK (length(signature) > 0),
    CONSTRAINT market_orders_condition_id_nonempty_check CHECK (length(condition_id) > 0)
);

CREATE INDEX IF NOT EXISTS market_orders_user_status_created_idx
ON market_orders (user_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS market_orders_market_status_side_price_idx
ON market_orders (market_id, status, side, price_bps, created_at DESC);

CREATE INDEX IF NOT EXISTS market_orders_wallet_status_created_idx
ON market_orders (wallet_address, status, created_at DESC);
