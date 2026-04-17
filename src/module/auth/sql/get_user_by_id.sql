SELECT id, email, username, display_name, avatar_url, created_at, updated_at
FROM users
WHERE id = $1
