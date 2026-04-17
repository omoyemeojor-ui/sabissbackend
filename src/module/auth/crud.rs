use uuid::Uuid;

use crate::{
    config::environment::Environment,
    config::db::DbPool,
    module::auth::error::AuthError,
    module::auth::model::{
        ACCOUNT_KIND_STELLAR_SMART_WALLET, UserProfileRecord, UserRecord, VerifiedGoogleToken,
        WALLET_STATUS_ACTIVE, WALLET_STATUS_PENDING_REGISTRATION, WalletChallengeRecord,
        WalletRecord,
    },
};

mod sql {
    pub const FIND_USER_BY_GOOGLE_SUB: &str = r#"
        SELECT
            u.id,
            u.email,
            u.username,
            u.display_name,
            u.avatar_url,
            u.created_at,
            u.updated_at
        FROM users u
        INNER JOIN google_identities g ON g.user_id = u.id
        WHERE g.google_sub = $1
    "#;
    pub const FIND_USER_BY_WALLET_ADDRESS: &str =
        include_str!("sql/find_user_by_wallet_address.sql");
    pub const GET_WALLET_FOR_USER: &str = include_str!("sql/get_wallet_for_user.sql");
    pub const GET_WALLET_CHALLENGE_BY_ID: &str = include_str!("sql/get_wallet_challenge_by_id.sql");
    pub const INSERT_WALLET_CHALLENGE: &str = include_str!("sql/insert_wallet_challenge.sql");
    pub const CONSUME_WALLET_CHALLENGE: &str = include_str!("sql/consume_wallet_challenge.sql");
    pub const INSERT_WALLET_USER: &str = include_str!("sql/insert_wallet_user.sql");
    pub const INSERT_WALLET_ACCOUNT: &str = include_str!("sql/insert_wallet_account.sql");
    pub const UPDATE_GOOGLE_USER: &str = r#"
        UPDATE users
        SET
            email = COALESCE($2, email),
            display_name = COALESCE($3, display_name),
            avatar_url = COALESCE($4, avatar_url),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, username, display_name, avatar_url, created_at, updated_at
    "#;
    pub const UPDATE_GOOGLE_IDENTITY: &str = r#"
        UPDATE google_identities
        SET
            email = $2,
            email_verified = $3,
            updated_at = NOW()
        WHERE user_id = $1
    "#;
    pub const INSERT_USER: &str = r#"
        INSERT INTO users (id, email, username, display_name, avatar_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, email, username, display_name, avatar_url, created_at, updated_at
    "#;
    pub const INSERT_GOOGLE_IDENTITY: &str = r#"
        INSERT INTO google_identities (user_id, google_sub, email, email_verified)
        VALUES ($1, $2, $3, $4)
    "#;
    pub const GET_USER_PROFILE_BY_ID: &str = r#"
        SELECT
            u.id,
            u.email,
            u.username,
            u.display_name,
            u.avatar_url,
            u.created_at,
            u.updated_at,
            w.wallet_address,
            w.network AS wallet_network,
            w.account_kind AS wallet_account_kind,
            w.wallet_status,
            w.wallet_standard,
            w.owner_provider AS wallet_owner_provider,
            w.owner_ref AS wallet_owner_ref,
            w.sponsor_address AS wallet_sponsor_address,
            w.relayer_kind AS wallet_relayer_kind,
            w.relayer_url AS wallet_relayer_url,
            w.web_auth_contract_id AS wallet_web_auth_contract_id,
            w.web_auth_domain AS wallet_web_auth_domain,
            w.deployed_at AS wallet_deployed_at,
            w.last_authenticated_at AS wallet_last_authenticated_at,
            w.created_at AS wallet_created_at
        FROM users u
        LEFT JOIN wallet_accounts w ON w.user_id = u.id
        WHERE u.id = $1
    "#;
    pub const UPSERT_GOOGLE_SMART_WALLET: &str = r#"
        INSERT INTO wallet_accounts (
            id,
            user_id,
            wallet_address,
            network,
            account_kind,
            wallet_status,
            wallet_standard,
            owner_provider,
            owner_ref,
            sponsor_address,
            relayer_kind,
            relayer_url,
            web_auth_contract_id,
            web_auth_domain,
            last_authenticated_at
        )
        VALUES (
            $1,
            $2,
            NULL,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9,
            $10,
            $11,
            $12,
            $13,
            $14,
            NOW()
        )
        ON CONFLICT (user_id) DO UPDATE
        SET
            network = EXCLUDED.network,
            account_kind = CASE
                WHEN wallet_accounts.wallet_status = $15 THEN wallet_accounts.account_kind
                ELSE EXCLUDED.account_kind
            END,
            wallet_status = CASE
                WHEN wallet_accounts.wallet_address IS NOT NULL THEN $15
                ELSE EXCLUDED.wallet_status
            END,
            wallet_standard = EXCLUDED.wallet_standard,
            owner_provider = EXCLUDED.owner_provider,
            owner_ref = EXCLUDED.owner_ref,
            sponsor_address = EXCLUDED.sponsor_address,
            relayer_kind = EXCLUDED.relayer_kind,
            relayer_url = EXCLUDED.relayer_url,
            web_auth_contract_id = COALESCE(EXCLUDED.web_auth_contract_id, wallet_accounts.web_auth_contract_id),
            web_auth_domain = COALESCE(EXCLUDED.web_auth_domain, wallet_accounts.web_auth_domain),
            last_authenticated_at = NOW()
        RETURNING
            wallet_address,
            network,
            account_kind,
            wallet_status,
            wallet_standard,
            owner_provider,
            owner_ref,
            sponsor_address,
            relayer_kind,
            relayer_url,
            web_auth_contract_id,
            web_auth_domain,
            deployed_at,
            last_authenticated_at,
            created_at
    "#;
    pub const REGISTER_SMART_WALLET: &str = r#"
        UPDATE wallet_accounts
        SET
            wallet_address = $2,
            wallet_standard = COALESCE($3, wallet_standard),
            relayer_url = COALESCE($4, relayer_url),
            web_auth_domain = COALESCE($5, web_auth_domain),
            wallet_status = $6,
            deployed_at = COALESCE(deployed_at, NOW()),
            last_authenticated_at = NOW()
        WHERE user_id = $1
          AND account_kind = $7
        RETURNING
            wallet_address,
            network,
            account_kind,
            wallet_status,
            wallet_standard,
            owner_provider,
            owner_ref,
            sponsor_address,
            relayer_kind,
            relayer_url,
            web_auth_contract_id,
            web_auth_domain,
            deployed_at,
            last_authenticated_at,
            created_at
    "#;
}

pub async fn upsert_google_user(
    pool: &DbPool,
    token: &VerifiedGoogleToken,
) -> Result<UserRecord, AuthError> {
    let mut tx = pool.begin().await?;

    if let Some(existing_user) = find_user_by_google_sub_tx(&mut tx, &token.google_sub).await? {
        let updated_user = sqlx::query_as::<_, UserRecord>(sql::UPDATE_GOOGLE_USER)
            .bind(existing_user.id)
            .bind(token.email.as_deref())
            .bind(token.display_name.as_deref())
            .bind(token.avatar_url.as_deref())
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query(sql::UPDATE_GOOGLE_IDENTITY)
            .bind(existing_user.id)
            .bind(token.email.as_deref())
            .bind(token.email_verified)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        return Ok(updated_user);
    }

    let user_id = Uuid::new_v4();
    let inserted_user = sqlx::query_as::<_, UserRecord>(sql::INSERT_USER)
        .bind(user_id)
        .bind(token.email.as_deref())
        .bind(Option::<&str>::None)
        .bind(token.display_name.as_deref())
        .bind(token.avatar_url.as_deref())
        .fetch_one(&mut *tx)
        .await?;

    let identity_result = sqlx::query(sql::INSERT_GOOGLE_IDENTITY)
        .bind(user_id)
        .bind(&token.google_sub)
        .bind(token.email.as_deref())
        .bind(token.email_verified)
        .execute(&mut *tx)
        .await;

    match identity_result {
        Ok(_) => {
            tx.commit().await?;
            Ok(inserted_user)
        }
        Err(error) if is_unique_violation(&error) => {
            tx.rollback().await?;
            find_user_by_google_sub(pool, &token.google_sub)
                .await?
                .ok_or_else(|| AuthError::unauthorized("user not found"))
        }
        Err(error) => Err(AuthError::from(error)),
    }
}

pub async fn get_wallet_for_user(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<WalletRecord>, AuthError> {
    sqlx::query_as::<_, WalletRecord>(sql::GET_WALLET_FOR_USER)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn ensure_google_smart_wallet(
    pool: &DbPool,
    env: &Environment,
    user_id: Uuid,
    google_sub: &str,
) -> Result<WalletRecord, AuthError> {
    sqlx::query_as::<_, WalletRecord>(sql::UPSERT_GOOGLE_SMART_WALLET)
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&env.network)
        .bind(ACCOUNT_KIND_STELLAR_SMART_WALLET)
        .bind(WALLET_STATUS_PENDING_REGISTRATION)
        .bind("sep-45")
        .bind("google_oidc")
        .bind(google_sub)
        .bind(&env.stellar_aa_sponsor_address)
        .bind(&env.stellar_aa_relayer_kind)
        .bind(env.stellar_aa_relayer_url.as_deref())
        .bind(env.sep45_web_auth_contract_id.as_deref())
        .bind(env.sep45_web_auth_domain.as_deref())
        .bind(WALLET_STATUS_ACTIVE)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn register_smart_wallet(
    pool: &DbPool,
    user_id: Uuid,
    wallet_address: &str,
    wallet_standard: Option<&str>,
    relayer_url: Option<&str>,
    web_auth_domain: Option<&str>,
) -> Result<WalletRecord, AuthError> {
    sqlx::query_as::<_, WalletRecord>(sql::REGISTER_SMART_WALLET)
        .bind(user_id)
        .bind(wallet_address)
        .bind(wallet_standard)
        .bind(relayer_url)
        .bind(web_auth_domain)
        .bind(WALLET_STATUS_ACTIVE)
        .bind(ACCOUNT_KIND_STELLAR_SMART_WALLET)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AuthError::forbidden("smart-wallet profile not found for user"))
}

pub async fn get_user_profile_by_id(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<UserProfileRecord>, AuthError> {
    sqlx::query_as::<_, UserProfileRecord>(sql::GET_USER_PROFILE_BY_ID)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn get_wallet_challenge_by_id(
    pool: &DbPool,
    challenge_id: Uuid,
) -> Result<Option<WalletChallengeRecord>, AuthError> {
    sqlx::query_as::<_, WalletChallengeRecord>(sql::GET_WALLET_CHALLENGE_BY_ID)
        .bind(challenge_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn create_wallet_challenge(
    pool: &DbPool,
    challenge_id: Uuid,
    wallet_address: &str,
    network: &str,
    nonce: &str,
    message: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<WalletChallengeRecord, AuthError> {
    sqlx::query_as::<_, WalletChallengeRecord>(sql::INSERT_WALLET_CHALLENGE)
        .bind(challenge_id)
        .bind(wallet_address)
        .bind(network)
        .bind(nonce)
        .bind(message)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn consume_wallet_challenge(
    pool: &DbPool,
    challenge_id: Uuid,
) -> Result<bool, AuthError> {
    let result = sqlx::query(sql::CONSUME_WALLET_CHALLENGE)
        .bind(challenge_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn find_user_by_wallet_address(
    pool: &DbPool,
    wallet_address: &str,
) -> Result<Option<UserRecord>, AuthError> {
    sqlx::query_as::<_, UserRecord>(sql::FIND_USER_BY_WALLET_ADDRESS)
        .bind(wallet_address)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn create_wallet_user(
    pool: &DbPool,
    username: &str,
    wallet_address: &str,
    network: &str,
) -> Result<UserRecord, AuthError> {
    let mut tx = pool.begin().await?;
    let user_id = Uuid::new_v4();

    let inserted_user = sqlx::query_as::<_, UserRecord>(sql::INSERT_WALLET_USER)
        .bind(user_id)
        .bind(username)
        .bind(username)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_unique_user_error)?;

    let wallet_insert = sqlx::query_as::<_, WalletRecord>(sql::INSERT_WALLET_ACCOUNT)
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(wallet_address)
        .bind(network)
        .fetch_one(&mut *tx)
        .await;

    match wallet_insert {
        Ok(_) => {
            tx.commit().await?;
            Ok(inserted_user)
        }
        Err(error) if unique_constraint(&error) == Some("wallet_accounts_address_key") => {
            tx.rollback().await?;
            find_user_by_wallet_address(pool, wallet_address)
                .await?
                .ok_or_else(|| AuthError::conflict("wallet already linked to another user"))
        }
        Err(error) => Err(AuthError::from(error)),
    }
}

async fn find_user_by_google_sub(
    pool: &DbPool,
    google_sub: &str,
) -> Result<Option<UserRecord>, AuthError> {
    sqlx::query_as::<_, UserRecord>(sql::FIND_USER_BY_GOOGLE_SUB)
        .bind(google_sub)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}

async fn find_user_by_google_sub_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    google_sub: &str,
) -> Result<Option<UserRecord>, AuthError> {
    sqlx::query_as::<_, UserRecord>(sql::FIND_USER_BY_GOOGLE_SUB)
        .bind(google_sub)
        .fetch_optional(&mut **tx)
        .await
        .map_err(AuthError::from)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505")
    )
}

fn unique_constraint(error: &sqlx::Error) -> Option<&str> {
    match error {
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505") =>
        {
            database_error.constraint()
        }
        _ => None,
    }
}

fn map_unique_user_error(error: sqlx::Error) -> AuthError {
    match unique_constraint(&error) {
        Some("users_username_key") => AuthError::conflict("username already taken"),
        _ => AuthError::from(error),
    }
}
