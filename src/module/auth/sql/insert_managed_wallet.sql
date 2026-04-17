INSERT INTO wallet_accounts (
    id,
    user_id,
    wallet_address,
    chain_id,
    account_kind,
    owner_address,
    owner_provider,
    owner_ref,
    factory_address,
    entry_point_address,
    owner_encrypted_private_key,
    owner_encryption_nonce,
    owner_key_version
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
ON CONFLICT (user_id) DO UPDATE
SET
    wallet_address = EXCLUDED.wallet_address,
    chain_id = EXCLUDED.chain_id,
    account_kind = EXCLUDED.account_kind,
    owner_address = EXCLUDED.owner_address,
    owner_provider = EXCLUDED.owner_provider,
    owner_ref = EXCLUDED.owner_ref,
    factory_address = EXCLUDED.factory_address,
    entry_point_address = EXCLUDED.entry_point_address,
    owner_encrypted_private_key = EXCLUDED.owner_encrypted_private_key,
    owner_encryption_nonce = EXCLUDED.owner_encryption_nonce,
    owner_key_version = EXCLUDED.owner_key_version
RETURNING
    wallet_address,
    chain_id,
    account_kind,
    owner_address,
    owner_provider,
    factory_address,
    entry_point_address,
    created_at
