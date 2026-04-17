use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, Method, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use reqwest::Client;
use serde::Serialize;
use sqlx::Executor;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    config::{db::DbPool, environment::Environment},
    module::{
        admin::route::router as admin_router, auth::route::router as auth_router,
        liquidity::route::{me_router as me_liquidity_router, public_router as public_liquidity_router},
        market::route::{me_router as me_market_router, public_router as public_market_router},
    },
};

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub env: Environment,
    pub http_client: Client,
}

pub fn build_router(state: AppState) -> Result<Router> {
    Ok(Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth_router(state.clone()))
        .nest("/admin", admin_router(state.clone()))
        .merge(public_market_router())
        .merge(public_liquidity_router())
        .merge(me_market_router(state.clone()))
        .merge(me_liquidity_router(state.clone()))
        .with_state(state.clone())
        .layer(build_cors_layer(&state.env)?)
        .layer(TraceLayer::new_for_http()))
}

fn build_cors_layer(env: &Environment) -> Result<CorsLayer> {
    let allowed_origins = env
        .cors_allowed_origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin)
                .with_context(|| format!("invalid CORS_ALLOWED_ORIGINS value `{origin}`"))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE]))
}

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.execute("SELECT 1").await {
        Ok(_) => (StatusCode::OK, Json(HealthResponse { status: "ok" })),
        Err(error) => {
            tracing::error!(?error, "database health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse { status: "degraded" }),
            )
        }
    }
}
