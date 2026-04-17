ALTER TABLE market_comments
ADD COLUMN IF NOT EXISTS parent_comment_id UUID REFERENCES market_comments(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS market_comments_parent_created_idx
ON market_comments (parent_comment_id, created_at ASC)
WHERE parent_comment_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS market_comment_likes (
    comment_id UUID NOT NULL REFERENCES market_comments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (comment_id, user_id)
);

CREATE INDEX IF NOT EXISTS market_comment_likes_user_created_idx
ON market_comment_likes (user_id, created_at DESC);
