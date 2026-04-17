-- Comprehensive Market and Event Tables

CREATE TABLE IF NOT EXISTS market_events (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    category_slug TEXT NOT NULL,
    subcategory_slug TEXT,
    tag_slugs TEXT[] DEFAULT '{}',
    image_url TEXT,
    summary_text TEXT,
    rules_text TEXT NOT NULL,
    context_text TEXT,
    additional_context TEXT,
    resolution_sources TEXT[] DEFAULT '{}',
    resolution_timezone TEXT NOT NULL DEFAULT 'UTC',
    starts_at TIMESTAMPTZ,
    sort_at TIMESTAMPTZ,
    featured BOOLEAN NOT NULL DEFAULT FALSE,
    breaking BOOLEAN NOT NULL DEFAULT FALSE,
    searchable BOOLEAN NOT NULL DEFAULT TRUE,
    visible BOOLEAN NOT NULL DEFAULT TRUE,
    hide_resolved_by_default BOOLEAN NOT NULL DEFAULT FALSE,
    group_key TEXT NOT NULL,
    series_key TEXT NOT NULL,
    event_id TEXT NOT NULL UNIQUE, -- The on-chain event ID (BytesN<32>)
    group_id TEXT NOT NULL,        -- The on-chain group ID
    series_id TEXT NOT NULL,       -- The on-chain series ID
    neg_risk BOOLEAN NOT NULL DEFAULT FALSE,
    oracle_address TEXT,
    publication_status TEXT NOT NULL DEFAULT 'Draft', -- Draft, Published
    published_tx_hash TEXT,
    created_by_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS markets (
    id UUID PRIMARY KEY,
    event_db_id UUID REFERENCES market_events(id) ON DELETE CASCADE,
    slug TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL,
    question TEXT NOT NULL,
    question_id TEXT NOT NULL,
    condition_id TEXT UNIQUE, -- Nullable if not yet published on-chain
    market_type TEXT NOT NULL, -- Binary, MultiOutcome, NegRisk
    outcome_count INTEGER NOT NULL,
    outcomes TEXT[] NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    publication_status TEXT NOT NULL DEFAULT 'Draft', -- Draft, Published
    trading_status TEXT NOT NULL DEFAULT 'Open', -- Open, Paused, Resolved
    metadata_hash TEXT,
    oracle_address TEXT NOT NULL,
    volume NUMERIC NOT NULL DEFAULT 0,
    liquidity NUMERIC NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS market_resolutions (
    market_id UUID PRIMARY KEY REFERENCES markets(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'None', -- None, Proposed, Disputed, Finalized
    proposed_winning_outcome INTEGER,
    final_winning_outcome INTEGER,
    payout_vector_hash TEXT,
    proposed_by_user_id UUID,
    proposed_at TIMESTAMPTZ,
    dispute_deadline TIMESTAMPTZ,
    notes TEXT,
    disputed_by_user_id UUID,
    disputed_at TIMESTAMPTZ,
    dispute_reason TEXT,
    finalized_by_user_id UUID,
    finalized_at TIMESTAMPTZ,
    emergency_resolved_by_user_id UUID,
    emergency_resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS market_outcomes (
    id UUID PRIMARY KEY,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    outcome_index INTEGER NOT NULL,
    name TEXT NOT NULL,
    probability NUMERIC NOT NULL DEFAULT 0,
    price NUMERIC NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (market_id, outcome_index)
);

CREATE INDEX IF NOT EXISTS markets_event_db_id_idx ON markets (event_db_id);
CREATE INDEX IF NOT EXISTS markets_trading_status_idx ON markets (trading_status);
CREATE INDEX IF NOT EXISTS markets_publication_status_idx ON markets (publication_status);
CREATE INDEX IF NOT EXISTS market_events_category_slug_idx ON market_events (category_slug);
CREATE INDEX IF NOT EXISTS market_events_slug_idx ON market_events (slug);
