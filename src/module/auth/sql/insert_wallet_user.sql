INSERT INTO users (id, username, display_name)
VALUES ($1, $2, $3)
RETURNING id, email, username, display_name, avatar_url, created_at, updated_at
