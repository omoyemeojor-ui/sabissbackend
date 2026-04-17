UPDATE users
SET
    email = COALESCE($2, email),
    display_name = COALESCE($3, display_name),
    avatar_url = COALESCE($4, avatar_url),
    updated_at = NOW()
WHERE id = $1
RETURNING id, email, username, display_name, avatar_url, created_at, updated_at
