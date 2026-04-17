ALTER TABLE wallet_accounts
ALTER COLUMN chain_id DROP NOT NULL;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS wallet_provider TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS provider_user_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS wallet_accounts_user_id_key
ON wallet_accounts (user_id);

CREATE UNIQUE INDEX IF NOT EXISTS wallet_accounts_address_key
ON wallet_accounts (wallet_address);

CREATE UNIQUE INDEX IF NOT EXISTS wallet_accounts_provider_user_id_key
ON wallet_accounts (wallet_provider, provider_user_id)
WHERE wallet_provider IS NOT NULL AND provider_user_id IS NOT NULL;
