CREATE TABLE IF NOT EXISTS admin_upload_assets (
    id UUID PRIMARY KEY,
    storage_provider TEXT NOT NULL,
    bucket_name TEXT NOT NULL,
    scope TEXT NOT NULL,
    file_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL CHECK (size_bytes >= 0),
    cid TEXT NOT NULL,
    ipfs_url TEXT NOT NULL,
    gateway_url TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT admin_upload_assets_storage_provider_check
        CHECK (storage_provider IN ('filebase_ipfs'))
);

CREATE INDEX IF NOT EXISTS admin_upload_assets_created_by_user_id_idx
ON admin_upload_assets (created_by_user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS admin_upload_assets_cid_idx
ON admin_upload_assets (cid);
