UPDATE market_events
SET publication_status = LOWER(publication_status)
WHERE publication_status IS NOT NULL;

UPDATE markets
SET publication_status = LOWER(publication_status)
WHERE publication_status IS NOT NULL;

UPDATE markets
SET trading_status = CASE LOWER(trading_status)
    WHEN 'open' THEN 'active'
    ELSE LOWER(trading_status)
END
WHERE trading_status IS NOT NULL;

ALTER TABLE market_events
ALTER COLUMN publication_status SET DEFAULT 'draft';

ALTER TABLE markets
ALTER COLUMN publication_status SET DEFAULT 'draft';

ALTER TABLE markets
ALTER COLUMN trading_status SET DEFAULT 'active';

ALTER TABLE market_events
DROP CONSTRAINT IF EXISTS market_events_publication_status_check;

ALTER TABLE markets
DROP CONSTRAINT IF EXISTS markets_publication_status_check;

ALTER TABLE markets
DROP CONSTRAINT IF EXISTS markets_trading_status_check;

ALTER TABLE market_events
ADD CONSTRAINT market_events_publication_status_check
CHECK (publication_status IN ('draft', 'published'));

ALTER TABLE markets
ADD CONSTRAINT markets_publication_status_check
CHECK (publication_status IN ('draft', 'published'));

ALTER TABLE markets
ADD CONSTRAINT markets_trading_status_check
CHECK (trading_status IN ('active', 'paused', 'resolved'));
