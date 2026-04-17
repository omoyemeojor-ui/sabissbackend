SELECT u.id, u.email, u.username, u.display_name, u.avatar_url, u.created_at, u.updated_at
FROM users u
INNER JOIN google_identities g ON g.user_id = u.id
WHERE g.google_sub = $1
