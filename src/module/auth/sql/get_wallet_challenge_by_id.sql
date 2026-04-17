SELECT id, wallet_address, network, nonce, message, expires_at, consumed_at, created_at
FROM wallet_challenges
WHERE id = $1
