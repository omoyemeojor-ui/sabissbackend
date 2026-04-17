-- Add events table
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE, -- Soroban BytesN<32>
    group_id TEXT NOT NULL,
    series_id TEXT NOT NULL,
    neg_risk BOOLEAN NOT NULL DEFAULT FALSE,
    title TEXT NOT NULL,
    description TEXT,
    image_url TEXT,
    category TEXT,
    status TEXT NOT NULL DEFAULT 'Open',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Update markets table to link to events and add metadata hash
ALTER TABLE markets ADD COLUMN IF NOT EXISTS event_id UUID REFERENCES events(id) ON DELETE SET NULL;
ALTER TABLE markets ADD COLUMN IF NOT EXISTS metadata_hash TEXT;
ALTER TABLE markets ADD COLUMN IF NOT EXISTS question_text TEXT;

-- Index for event lookup
CREATE INDEX IF NOT EXISTS markets_event_id_idx ON markets (event_id);
