use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode, response::Response},
};
use sabissbackend::{
    app::{AppState, build_router},
    config::{db::create_pool, environment::Environment},
    module::auth::schema::{WalletChallengeRequest, WalletChallengeResponse},
};
use serde_json::{Value, json};
use tower::util::ServiceExt;
use reqwest::Client;

#[tokio::test]
async fn test_health_check() -> Result<()> {
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
                .uri("/health")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["status"], "ok");

    Ok(())
}

#[tokio::test]
async fn test_wallet_challenge_flow() -> Result<()> {
    let env = Environment::load()?;
    let db = create_pool(&env).await?;
    let state = AppState {
        db,
        env: env.clone(),
        http_client: Client::new(),
    };
    let app = build_router(state)?;

    // 1. Request a challenge
    let wallet_address = "GC6UJTOU4SL2VU3EQ5S4P6W32MEAJAZL4TI6Y7AQZUTZXYFA6FICBWQ3";
    let challenge_req = WalletChallengeRequest {
        wallet_address: wallet_address.to_string(),
    };

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/wallet/challenge")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&challenge_req)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), 2048).await?;
    let challenge_res: WalletChallengeResponse = serde_json::from_slice(&body)?;
    
    assert!(!challenge_res.message.is_empty());
    assert!(challenge_res.message.contains(wallet_address));
    
    Ok(())
}

#[tokio::test]
async fn test_google_sign_in_config() -> Result<()> {
    let env = Environment::load()?;
    
    // Verify that the environment variables are correctly loaded
    assert!(env.google_client_id.is_some(), "GOOGLE_CLIENT_ID should be set in .env");
    assert_eq!(env.google_client_id.as_ref().unwrap(), "153387979068-g9sg813uuih831nsd1a3480trjlqnn7a.apps.googleusercontent.com");
    
    let db = create_pool(&env).await?;
    let state = AppState {
        db,
        env: env.clone(),
        http_client: Client::new(),
    };
    let app = build_router(state)?;

    // Test with an invalid credential to see if it reaches the verification logic
    let google_req = json!({
        "credential": "invalid-token",
        "g_csrf_token": "test-csrf",
        "client_id": "153387979068-g9sg813uuih831nsd1a3480trjlqnn7a.apps.googleusercontent.com"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/google/sign-in")
                .header("content-type", "application/json")
                .header("cookie", "g_csrf_token=test-csrf")
                .body(Body::from(serde_json::to_vec(&google_req)?))?,
        )
        .await?;

    // It should fail because the token is invalid, but it confirms the route and config are working
    // 401 Unauthorized or 400 Bad Request depending on implementation
    assert!(response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::BAD_REQUEST);
    
    Ok(())
}
