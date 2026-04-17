CREATE INDEX IF NOT EXISTS market_events_public_visibility_sort_idx
ON market_events (publication_status, visible, featured, breaking, sort_at DESC, starts_at DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS market_events_public_category_idx
ON market_events (category_slug, subcategory_slug)
WHERE publication_status = 'published' AND visible = TRUE;

CREATE INDEX IF NOT EXISTS market_events_public_slug_idx
ON market_events (slug)
WHERE publication_status = 'published' AND visible = TRUE;

CREATE INDEX IF NOT EXISTS market_events_tag_slugs_gin_idx
ON market_events USING GIN (tag_slugs);

CREATE INDEX IF NOT EXISTS markets_public_filter_sort_idx
ON markets (publication_status, trading_status, created_at DESC, sort_order ASC);

CREATE INDEX IF NOT EXISTS markets_public_event_sort_idx
ON markets (event_db_id, publication_status, sort_order ASC, created_at ASC);
