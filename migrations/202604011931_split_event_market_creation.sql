ALTER TABLE market_events
ALTER COLUMN oracle_address DROP NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS markets_event_db_id_sort_order_key
ON markets (event_db_id, sort_order);
