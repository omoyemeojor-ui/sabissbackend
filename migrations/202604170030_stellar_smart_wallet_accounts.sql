ALTER TABLE wallet_accounts
ALTER COLUMN wallet_address DROP NOT NULL;

DROP INDEX IF EXISTS wallet_accounts_address_key;

CREATE UNIQUE INDEX IF NOT EXISTS wallet_accounts_address_key
ON wallet_accounts (wallet_address)
WHERE wallet_address IS NOT NULL;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS account_kind TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS wallet_status TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS wallet_standard TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS sponsor_address TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS relayer_kind TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS relayer_url TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS factory_contract_id TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS web_auth_contract_id TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS web_auth_domain TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS deployed_at TIMESTAMPTZ;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS last_authenticated_at TIMESTAMPTZ;

UPDATE wallet_accounts
SET account_kind = 'classic_account'
WHERE account_kind IS NULL;

UPDATE wallet_accounts
SET wallet_status = CASE
    WHEN wallet_address IS NULL THEN 'pending_registration'
    ELSE 'active'
END
WHERE wallet_status IS NULL;

ALTER TABLE wallet_accounts
ALTER COLUMN account_kind SET NOT NULL;

ALTER TABLE wallet_accounts
ALTER COLUMN account_kind SET DEFAULT 'classic_account';

ALTER TABLE wallet_accounts
ALTER COLUMN wallet_status SET NOT NULL;

ALTER TABLE wallet_accounts
ALTER COLUMN wallet_status SET DEFAULT 'active';
