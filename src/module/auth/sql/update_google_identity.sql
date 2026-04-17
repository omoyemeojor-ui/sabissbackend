UPDATE google_identities
SET
    email = $2,
    email_verified = $3,
    updated_at = NOW()
WHERE user_id = $1
