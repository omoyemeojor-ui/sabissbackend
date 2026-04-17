CREATE INDEX IF NOT EXISTS market_events_public_search_tsv_idx
ON market_events
USING GIN (to_tsvector('simple', coalesce(title, '') || ' ' || coalesce(slug, '')))
WHERE publication_status = 'published' AND visible = TRUE AND searchable = TRUE;

CREATE INDEX IF NOT EXISTS markets_public_search_tsv_idx
ON markets
USING GIN (to_tsvector('simple', coalesce(label, '') || ' ' || coalesce(question, '') || ' ' || coalesce(slug, '')))
WHERE publication_status = 'published';
