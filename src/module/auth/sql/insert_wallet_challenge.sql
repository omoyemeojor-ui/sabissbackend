INSERT INTO wallet_challenges (
    id,
    wallet_address,
    network,
    nonce,
    message,
    expires_at
)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING id, wallet_address, network, nonce, message, expires_at, consumed_at, created_at
