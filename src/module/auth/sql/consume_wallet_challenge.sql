UPDATE wallet_challenges
SET consumed_at = NOW()
WHERE id = $1
  AND consumed_at IS NULL
  AND expires_at > NOW()
