use crate::{
    app::AppState,
    module::auth::{crud, error::AuthError, model::UserRecord, model::VerifiedGoogleToken},
};

pub async fn ensure_google_user_smart_wallet(
    state: &AppState,
    user: &UserRecord,
    verified: &VerifiedGoogleToken,
) -> Result<(), AuthError> {
    crud::ensure_google_smart_wallet(&state.db, &state.env, user.id, &verified.google_sub).await?;
    Ok(())
}
