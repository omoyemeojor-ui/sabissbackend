use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{
        admin::model::AdminProfile,
        auth::{crud as auth_crud, error::AuthError},
    },
};

pub async fn get_admin_profile(pool: &DbPool, user_id: Uuid) -> Result<AdminProfile, AuthError> {
    let profile = auth_crud::get_user_profile_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("invalid session"))?;
    let (user, wallet) = profile.into_parts();
    let wallet = wallet.ok_or_else(|| AuthError::forbidden("admin wallet not linked"))?;

    Ok(AdminProfile { user, wallet })
}
