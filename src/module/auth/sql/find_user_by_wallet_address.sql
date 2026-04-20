SELECT u.id, u.email, u.username, u.display_name, u.avatar_url, u.created_at, u.updated_at
FROM users u
INNER JOIN wallet_accounts w ON w.user_id = u.id
WHERE w.wallet_address = $1
   OR w.owner_ref = $1
