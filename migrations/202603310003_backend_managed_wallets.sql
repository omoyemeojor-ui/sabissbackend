DROP TABLE IF EXISTS wallet_challenges;

DROP INDEX IF EXISTS wallet_accounts_provider_user_id_key;
DROP INDEX IF EXISTS wallet_accounts_address_chain_id_key;

ALTER TABLE wallet_accounts
DROP COLUMN IF EXISTS wallet_provider;

ALTER TABLE wallet_accounts
DROP COLUMN IF EXISTS provider_user_id;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS encrypted_private_key TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS encryption_nonce TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS key_version INTEGER;

UPDATE wallet_accounts
SET chain_id = 10143
WHERE chain_id IS NULL;

ALTER TABLE wallet_accounts
ALTER COLUMN chain_id SET DEFAULT 10143;

CREATE UNIQUE INDEX IF NOT EXISTS wallet_accounts_user_id_key
ON wallet_accounts (user_id);

CREATE UNIQUE INDEX IF NOT EXISTS wallet_accounts_address_key
ON wallet_accounts (wallet_address);
