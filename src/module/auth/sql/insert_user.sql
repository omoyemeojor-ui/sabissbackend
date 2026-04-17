INSERT INTO users (id, email, username, display_name, avatar_url)
VALUES ($1, $2, $3, $4, $5)
RETURNING id, email, username, display_name, avatar_url, created_at, updated_at
