ALTER TABLE markets
ADD COLUMN IF NOT EXISTS trading_status TEXT NOT NULL DEFAULT 'active';

ALTER TABLE markets
DROP CONSTRAINT IF EXISTS markets_trading_status_check;

ALTER TABLE markets
ADD CONSTRAINT markets_trading_status_check
CHECK (trading_status IN ('active', 'paused'));
