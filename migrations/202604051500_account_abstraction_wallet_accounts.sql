ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS account_kind TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS owner_address TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS owner_provider TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS owner_ref TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS factory_address TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS entry_point_address TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS owner_encrypted_private_key TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS owner_encryption_nonce TEXT;

ALTER TABLE wallet_accounts
ADD COLUMN IF NOT EXISTS owner_key_version INTEGER;

UPDATE wallet_accounts
SET account_kind = 'external_eoa'
WHERE account_kind IS NULL;

ALTER TABLE wallet_accounts
ALTER COLUMN account_kind SET NOT NULL;

ALTER TABLE wallet_accounts
ALTER COLUMN account_kind SET DEFAULT 'external_eoa';

ALTER TABLE wallet_accounts
DROP COLUMN IF EXISTS encrypted_private_key;

ALTER TABLE wallet_accounts
DROP COLUMN IF EXISTS encryption_nonce;

ALTER TABLE wallet_accounts
DROP COLUMN IF EXISTS key_version;
