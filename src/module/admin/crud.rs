use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        admin::model::{AdminProfile, AdminUploadAssetRecord, NewAdminUploadAssetRecord},
        auth::{crud as auth_crud, error::AuthError},
    },
};

mod sql {
    pub const INSERT_ADMIN_UPLOAD_ASSET: &str = r#"
        INSERT INTO admin_upload_assets (
            id,
            storage_provider,
            bucket_name,
            scope,
            file_name,
            content_type,
            size_bytes,
            cid,
            ipfs_url,
            gateway_url,
            created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING
            id,
            storage_provider,
            bucket_name,
            scope,
            file_name,
            content_type,
            size_bytes,
            cid,
            ipfs_url,
            gateway_url,
            created_by_user_id,
            created_at
    "#;
}

pub async fn get_admin_profile(pool: &DbPool, user_id: Uuid) -> Result<AdminProfile, AuthError> {
    let profile = auth_crud::get_user_profile_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("invalid session"))?;
    let (user, wallet) = profile.into_parts();
    let wallet = wallet.ok_or_else(|| AuthError::forbidden("admin wallet not linked"))?;

    Ok(AdminProfile { user, wallet })
}

pub async fn create_admin_upload_asset(
    pool: &DbPool,
    record: NewAdminUploadAssetRecord,
) -> Result<AdminUploadAssetRecord, AuthError> {
    sqlx::query_as::<_, AdminUploadAssetRecord>(sql::INSERT_ADMIN_UPLOAD_ASSET)
        .bind(record.id)
        .bind(record.storage_provider)
        .bind(record.bucket_name)
        .bind(record.scope)
        .bind(record.file_name)
        .bind(record.content_type)
        .bind(record.size_bytes)
        .bind(record.cid)
        .bind(record.ipfs_url)
        .bind(record.gateway_url)
        .bind(record.created_by_user_id)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}
