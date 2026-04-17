ALTER TABLE markets
DROP CONSTRAINT IF EXISTS markets_trading_status_check;

ALTER TABLE markets
ADD CONSTRAINT markets_trading_status_check
CHECK (trading_status IN ('active', 'paused', 'resolved'));

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
