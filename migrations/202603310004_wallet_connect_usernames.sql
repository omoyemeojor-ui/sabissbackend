ALTER TABLE users
ADD COLUMN IF NOT EXISTS username TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS users_username_key
ON users (username)
WHERE username IS NOT NULL;

CREATE TABLE IF NOT EXISTS wallet_challenges (
    id UUID PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    chain_id BIGINT NOT NULL,
    nonce TEXT NOT NULL,
    message TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS wallet_challenges_wallet_address_idx
ON wallet_challenges (wallet_address, created_at DESC);
