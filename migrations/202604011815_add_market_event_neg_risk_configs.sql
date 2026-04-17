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
