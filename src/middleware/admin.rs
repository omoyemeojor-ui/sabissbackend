use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    app::AppState,
    module::auth::{crud, error::AuthError},
    service::jwt::authenticate_headers,
};

pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let authenticated_user = authenticate_headers(request.headers(), &state.env)?;
    let wallet = crud::get_wallet_for_user(&state.db, authenticated_user.user_id)
        .await?
        .ok_or_else(|| AuthError::forbidden("admin wallet not linked"))?;
    let is_admin = wallet
        .wallet_address
        .as_deref()
        .is_some_and(|value| state.env.is_admin_wallet(value))
        || wallet
            .owner_ref
            .as_deref()
            .is_some_and(|value| state.env.is_admin_wallet(value));

    if !is_admin {
        return Err(AuthError::forbidden("admin access required"));
    }

    request.extensions_mut().insert(authenticated_user);
    Ok(next.run(request).await)
}
