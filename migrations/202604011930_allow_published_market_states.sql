ALTER TABLE market_events
DROP CONSTRAINT IF EXISTS market_events_publication_status_check;

ALTER TABLE market_events
ADD CONSTRAINT market_events_publication_status_check
CHECK (publication_status IN ('draft', 'published'));

ALTER TABLE markets
DROP CONSTRAINT IF EXISTS markets_publication_status_check;

ALTER TABLE markets
ADD CONSTRAINT markets_publication_status_check
CHECK (publication_status IN ('draft', 'published'));
