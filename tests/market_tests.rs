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
use serde_json::Value;
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

async fn seed_published_market(state: &AppState, created_by_user_id: Uuid) -> Result<SeededMarket> {
    let suffix = Uuid::new_v4().simple().to_string();
    let event_id = Uuid::new_v4();
    let market_id = Uuid::new_v4();
    let category_slug = format!("sports-{suffix}");
    let event_slug = format!("event-{suffix}");
    let market_slug = format!("market-{suffix}");
    let condition_id = format!("0x{:064x}", 1_u8);

    let event = NewMarketEventRecord {
        id: event_id,
        title: format!("Event {suffix}"),
        slug: event_slug.clone(),
        category_slug: category_slug.clone(),
        subcategory_slug: Some(format!("subcategory-{suffix}")),
        tag_slugs: vec![format!("tag-{suffix}")],
        image_url: None,
        summary_text: Some("summary".to_owned()),
        rules_text: "rules".to_owned(),
        context_text: Some("context".to_owned()),
        additional_context: None,
        resolution_sources: vec!["source".to_owned()],
        resolution_timezone: "UTC".to_owned(),
        starts_at: Some(Utc::now()),
        sort_at: Some(Utc::now()),
        featured: true,
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
        slug: market_slug.clone(),
        label: format!("Label {suffix}"),
        question: format!("Question {suffix}?"),
        question_id: format!("question-{suffix}"),
        condition_id: Some(condition_id.clone()),
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

    Ok(SeededMarket {
        event_id,
        market_id,
        category_slug,
        market_slug,
        condition_id,
    })
}

struct SeededMarket {
    event_id: Uuid,
    market_id: Uuid,
    category_slug: String,
    market_slug: String,
    condition_id: String,
}

#[tokio::test]
async fn market_public_collection_endpoints_return_structured_json() -> Result<()> {
    let state = build_test_state().await?;
    let app = build_router(state)?;

    for path in [
        "/events",
        "/categories",
        "/tags",
        "/markets/home",
        "/markets",
        "/markets/search?q=test",
    ] {
        let (status, json) = request_json(app.clone(), Method::GET, path, None, None).await?;
        assert_eq!(status, StatusCode::OK, "path {path}");
        assert!(json.is_object(), "expected object response for {path}, got {json}");
    }

    Ok(())
}

#[tokio::test]
async fn market_public_item_endpoints_return_expected_shapes_for_seeded_market() -> Result<()> {
    let state = build_test_state().await?;
    let (creator_user_id, _) = create_session_user(
        &state,
        "market_creator",
        "GCREATORMARKETTESTWALLET000000000000000000000000000000000000000001",
    )
    .await?;
    let seeded = seed_published_market(&state, creator_user_id).await?;
    let app = build_router(state)?;

    for (path, expected_key) in [
        (format!("/events/{}", seeded.event_id), "event"),
        (format!("/events/{}/markets", seeded.event_id), "markets"),
        (format!("/categories/{}", seeded.category_slug), "category"),
        (format!("/markets/{}", seeded.market_id), "market"),
        (format!("/markets/slug/{}", seeded.market_slug), "market"),
        (
            format!("/markets/by-condition/{}", seeded.condition_id),
            "market",
        ),
        (format!("/markets/{}/outcomes", seeded.market_id), "outcomes"),
        (format!("/markets/{}/activity", seeded.market_id), "items"),
        (format!("/markets/{}/resolution", seeded.market_id), "resolution"),
        (format!("/markets/{}/related", seeded.market_id), "related"),
        (format!("/markets/{}/price-history", seeded.market_id), "points"),
        (format!("/markets/{}/trades", seeded.market_id), "trades"),
    ] {
        let (status, json) = request_json(app.clone(), Method::GET, &path, None, None).await?;
        assert_eq!(status, StatusCode::OK, "path {path} returned {json}");
        assert!(json.get(expected_key).is_some(), "missing `{expected_key}` in {path}: {json}");
    }

    Ok(())
}

#[tokio::test]
async fn market_public_resource_endpoints_return_not_found_for_missing_resources() -> Result<()> {
    let state = build_test_state().await?;
    let app = build_router(state)?;
    let missing_uuid = Uuid::new_v4();
    let missing_condition = format!("0x{:064x}", 2_u8);

    for path in [
        format!("/events/{missing_uuid}"),
        format!("/events/{missing_uuid}/markets"),
        "/categories/missing-category".to_owned(),
        format!("/markets/{missing_uuid}"),
        "/markets/slug/missing-market".to_owned(),
        format!("/markets/by-condition/{missing_condition}"),
        format!("/markets/{missing_uuid}/liquidity"),
        format!("/markets/{missing_uuid}/resolution"),
        format!("/markets/{missing_uuid}/related"),
        format!("/markets/{missing_uuid}/outcomes"),
        format!("/markets/{missing_uuid}/activity"),
        format!("/markets/{missing_uuid}/quote"),
        format!("/markets/{missing_uuid}/price-history"),
        format!("/markets/{missing_uuid}/trades"),
        format!("/markets/{missing_uuid}/orderbook"),
    ] {
        let (status, json) = request_json(app.clone(), Method::GET, &path, None, None).await?;
        assert_eq!(status, StatusCode::NOT_FOUND, "path {path} returned {json}");
    }

    Ok(())
}

#[tokio::test]
async fn market_trade_endpoints_require_authentication() -> Result<()> {
    let state = build_test_state().await?;
    let app = build_router(state)?;
    let market_id = Uuid::new_v4();

    for (method, path) in [
        (Method::POST, format!("/markets/{market_id}/buy")),
        (Method::POST, format!("/markets/{market_id}/sell")),
        (Method::POST, format!("/markets/{market_id}/split")),
        (Method::POST, format!("/markets/{market_id}/merge")),
    ] {
        let (status, json) = request_json(app.clone(), method, &path, None, None).await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "path {path} returned {json}");
    }

    Ok(())
}

#[tokio::test]
async fn market_admin_endpoints_require_authentication() -> Result<()> {
    let state = build_test_state().await?;
    let app = build_router(state)?;
    let market_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();

    for (method, path) in [
        (Method::POST, "/admin/market-series/coinbase".to_owned()),
        (Method::POST, "/admin/markets".to_owned()),
        (Method::PATCH, format!("/admin/markets/{market_id}")),
        (Method::POST, format!("/admin/markets/{market_id}/prices")),
        (
            Method::POST,
            format!("/admin/markets/{market_id}/auto-resolve/coinbase"),
        ),
        (
            Method::POST,
            format!("/admin/markets/{market_id}/liquidity/bootstrap"),
        ),
        (Method::POST, format!("/admin/markets/{market_id}/pause")),
        (Method::POST, format!("/admin/markets/{market_id}/unpause")),
        (
            Method::POST,
            format!("/admin/markets/{market_id}/resolution/propose"),
        ),
        (
            Method::POST,
            format!("/admin/markets/{market_id}/resolution/dispute"),
        ),
        (
            Method::POST,
            format!("/admin/markets/{market_id}/resolution/finalize"),
        ),
        (
            Method::POST,
            format!("/admin/markets/{market_id}/resolution/emergency"),
        ),
        (Method::GET, "/admin/events".to_owned()),
        (Method::POST, "/admin/events".to_owned()),
        (Method::GET, format!("/admin/events/{event_id}")),
        (Method::POST, format!("/admin/events/{event_id}/publish")),
        (Method::GET, format!("/admin/events/{event_id}/markets")),
        (Method::POST, format!("/admin/events/{event_id}/markets")),
        (
            Method::POST,
            format!("/admin/events/{event_id}/markets/publish"),
        ),
        (
            Method::POST,
            format!("/admin/events/{event_id}/markets/ladders"),
        ),
        (
            Method::POST,
            format!("/admin/events/{event_id}/liquidity/bootstrap"),
        ),
        (
            Method::POST,
            format!("/admin/events/{event_id}/neg-risk/register"),
        ),
    ] {
        let (status, json) = request_json(app.clone(), method, &path, None, None).await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "path {path} returned {json}");
    }

    Ok(())
}
