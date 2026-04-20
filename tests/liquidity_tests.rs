use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use reqwest::Client;
use sabissbackend::{
    app::{AppState, build_router},
    config::{db::create_pool, environment::Environment},
    module::{
        auth::crud as auth_crud,
        market::{
            crud as market_crud,
            model::{NewMarketEventRecord, NewMarketRecord},
        },
    },
    service::jwt::create_session_token,
};
use serde_json::{Value, json};
use std::sync::Once;
use tower::util::ServiceExt;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

static TEST_TRACING: Once = Once::new();

fn init_test_tracing() {
    TEST_TRACING.call_once(|| {
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("sabissbackend=debug")),
            )
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .init();
    });
}

async fn build_test_state() -> Result<AppState> {
    init_test_tracing();
    let env = Environment::load()?;
    let db = create_pool(&env).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    Ok(AppState {
        db,
        env,
        http_client: Client::new(),
    })
}

async fn request_json(
    app: axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Result<(StatusCode, Value)> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }

    let request = builder.body(match body {
        Some(value) => Body::from(serde_json::to_vec(&value)?),
        None => Body::empty(),
    })?;
    let response = app.oneshot(request).await?;
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 128).await?;
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Ok((status, json))
}

async fn create_session_user(
    state: &AppState,
    username_prefix: &str,
    wallet_address: &str,
) -> Result<(Uuid, String)> {
    let username = format!("{username_prefix}_{}", Uuid::new_v4().simple());
    let user = auth_crud::create_wallet_user(&state.db, &username, wallet_address, &state.env.network)
        .await?;
    Ok((user.id, create_session_token(&state.env, &user)?))
}

async fn seed_published_market(state: &AppState, created_by_user_id: Uuid) -> Result<(Uuid, Uuid)> {
    let suffix = Uuid::new_v4().simple().to_string();
    let event_id = Uuid::new_v4();
    let market_id = Uuid::new_v4();

    let event = NewMarketEventRecord {
        id: event_id,
        title: format!("Liquidity Event {suffix}"),
        slug: format!("liquidity-event-{suffix}"),
        category_slug: format!("liquidity-category-{suffix}"),
        subcategory_slug: None,
        tag_slugs: vec![format!("liquidity-tag-{suffix}")],
        image_url: None,
        summary_text: Some("summary".to_owned()),
        rules_text: "rules".to_owned(),
        context_text: None,
        additional_context: None,
        resolution_sources: vec!["source".to_owned()],
        resolution_timezone: "UTC".to_owned(),
        starts_at: Some(Utc::now()),
        sort_at: Some(Utc::now()),
        featured: false,
        breaking: false,
        searchable: true,
        visible: true,
        hide_resolved_by_default: false,
        group_key: format!("group-{suffix}"),
        series_key: format!("series-{suffix}"),
        event_id: format!("event-id-{suffix}"),
        group_id: format!("group-id-{suffix}"),
        series_id: format!("series-id-{suffix}"),
        neg_risk: false,
        oracle_address: Some("oracle".to_owned()),
        publication_status: "published".to_owned(),
        published_tx_hash: Some("tx-hash".to_owned()),
        created_by_user_id,
    };

    let market = NewMarketRecord {
        id: market_id,
        event_db_id: event_id,
        slug: format!("liquidity-market-{suffix}"),
        label: format!("Liquidity Market {suffix}"),
        question: format!("Liquidity Question {suffix}?"),
        question_id: format!("liquidity-question-{suffix}"),
        condition_id: None,
        market_type: "binary".to_owned(),
        outcome_count: 2,
        outcomes: vec!["Yes".to_owned(), "No".to_owned()],
        end_time: Utc::now() + Duration::days(7),
        sort_order: 0,
        publication_status: "published".to_owned(),
        trading_status: "active".to_owned(),
        metadata_hash: None,
        oracle_address: "oracle".to_owned(),
    };

    market_crud::create_market_bundle(&state.db, &event, &market).await?;
    Ok((event_id, market_id))
}

#[tokio::test]
async fn liquidity_endpoints_require_auth_or_return_expected_shapes() -> Result<()> {
    let state = build_test_state().await?;
    let (user_id, token) = create_session_user(
        &state,
        "liquidity_user",
        "GLIQUIDITYTESTWALLET000000000000000000000000000000000000000000001",
    )
    .await?;
    let (event_id, market_id) = seed_published_market(&state, user_id).await?;
    let app = build_router(state)?;

    let (status, json) = request_json(
        app.clone(),
        Method::GET,
        &format!("/events/{event_id}/liquidity"),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("event").is_some());
    assert!(json.get("liquidity").is_some());

    for path in [
        format!("/me/liquidity/markets/{market_id}"),
        format!("/me/liquidity/events/{event_id}"),
    ] {
        let (status, json) =
            request_json(app.clone(), Method::GET, &path, Some(&token), None).await?;
        assert_eq!(status, StatusCode::OK, "path {path} returned {json}");
        assert!(json.get("wallet_address").is_some(), "path {path} returned {json}");
    }

    let (status, json) = request_json(
        app.clone(),
        Method::GET,
        &format!("/me/liquidity/events/{event_id}"),
        Some(&token),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json.get("markets")
            .and_then(Value::as_array)
            .map(|markets| markets.len()),
        Some(0)
    );

    for path in [
        format!("/me/liquidity/markets/{market_id}"),
        format!("/me/liquidity/events/{event_id}"),
        format!("/me/liquidity/markets/{market_id}/deposit-inventory"),
        format!("/me/liquidity/markets/{market_id}/deposit-collateral"),
        format!("/me/liquidity/markets/{market_id}/remove"),
        format!("/me/liquidity/markets/{market_id}/withdraw-inventory"),
        format!("/me/liquidity/markets/{market_id}/withdraw-collateral"),
    ] {
        let method = if path.contains("/deposit-")
            || path.contains("/remove")
            || path.contains("/withdraw-")
        {
            Method::POST
        } else {
            Method::GET
        };
        let (status, json) = request_json(app.clone(), method, &path, None, None).await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "path {path} returned {json}");
    }

    for (path, body) in [
        (
            format!("/me/liquidity/markets/{market_id}/deposit-inventory"),
            json!({"liquidity": {"yes_amount": "1000000", "no_amount": "1000000"}}),
        ),
        (
            format!("/me/liquidity/markets/{market_id}/deposit-collateral"),
            json!({"liquidity": {"amount": "1000000"}}),
        ),
        (
            format!("/me/liquidity/markets/{market_id}/remove"),
            json!({"liquidity": {"yes_amount": "1000000", "no_amount": "1000000"}}),
        ),
        (
            format!("/me/liquidity/markets/{market_id}/withdraw-inventory"),
            json!({"liquidity": {"yes_amount": "1000000", "no_amount": "1000000"}}),
        ),
        (
            format!("/me/liquidity/markets/{market_id}/withdraw-collateral"),
            json!({"liquidity": {"amount": "1000000"}}),
        ),
    ] {
        let (status, json) =
            request_json(app.clone(), Method::POST, &path, Some(&token), Some(body)).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "path {path} returned {json}");
        assert_eq!(
            json.get("error").and_then(Value::as_str),
            Some("market is not published on-chain"),
            "path {path} returned {json}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn liquidity_public_missing_event_returns_not_found() -> Result<()> {
    let state = build_test_state().await?;
    let app = build_router(state)?;

    let (status, json) = request_json(
        app,
        Method::GET,
        &format!("/events/{}/liquidity", Uuid::new_v4()),
        None,
        None,
    )
    .await?;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json.get("error").is_some());
    Ok(())
}
