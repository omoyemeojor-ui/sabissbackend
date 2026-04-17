use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sabissbackend::{
    app::{AppState, build_router},
    config::{db::create_pool, environment::Environment},
};
use serde_json::Value;
use tower::util::ServiceExt;
use reqwest::Client;

#[tokio::test]
async fn test_list_markets_empty() -> Result<()> {
    let env = Environment::load()?;
    let db = create_pool(&env).await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    
    let state = AppState {
        db,
        env: env.clone(),
        http_client: Client::new(),
    };
    let app = build_router(state)?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/markets")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), 1024 * 10).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert!(json.is_array());

    Ok(())
}

#[tokio::test]
async fn test_markets_home() -> Result<()> {
    let env = Environment::load()?;
    let db = create_pool(&env).await?;
    
    let state = AppState {
        db,
        env: env.clone(),
        http_client: Client::new(),
    };
    let app = build_router(state)?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/markets/home")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), 1024 * 10).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert!(json.is_array());

    Ok(())
}
