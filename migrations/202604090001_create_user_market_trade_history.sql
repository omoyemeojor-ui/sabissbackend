CREATE TABLE IF NOT EXISTS user_market_trade_history (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES market_events(id) ON DELETE CASCADE,
    wallet_address TEXT NOT NULL,
    execution_source TEXT NOT NULL,
    action TEXT NOT NULL,
    outcome_index INTEGER NOT NULL,
    price_bps INTEGER NOT NULL,
    token_amount TEXT NOT NULL,
    usdc_amount TEXT NOT NULL,
    tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT user_market_trade_history_execution_source_check CHECK (
        execution_source IN ('market_trade')
    ),
    CONSTRAINT user_market_trade_history_action_check CHECK (
        action IN ('buy', 'sell')
    ),
    CONSTRAINT user_market_trade_history_outcome_index_check CHECK (
        outcome_index IN (0, 1)
    ),
    CONSTRAINT user_market_trade_history_price_bps_check CHECK (
        price_bps >= 0 AND price_bps <= 10000
    ),
    CONSTRAINT user_market_trade_history_token_amount_nonempty_check CHECK (
        length(token_amount) > 0
    ),
    CONSTRAINT user_market_trade_history_usdc_amount_nonempty_check CHECK (
        length(usdc_amount) > 0
    )
);

CREATE INDEX IF NOT EXISTS user_market_trade_history_user_created_idx
ON user_market_trade_history (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS user_market_trade_history_market_created_idx
ON user_market_trade_history (market_id, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS user_market_trade_history_user_tx_action_outcome_idx
ON user_market_trade_history (user_id, market_id, tx_hash, action, outcome_index)
WHERE tx_hash IS NOT NULL;
