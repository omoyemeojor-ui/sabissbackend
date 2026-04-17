use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{app::AppState, module::auth::error::AuthError, service::jwt::authenticate_headers};

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let authenticated_user = authenticate_headers(request.headers(), &state.env)?;
    request.extensions_mut().insert(authenticated_user);

    Ok(next.run(request).await)
}
