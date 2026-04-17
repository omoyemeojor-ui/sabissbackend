use uuid::Uuid;

use crate::{
    config::db::DbPool,
    module::{auth::error::AuthError, liquidity::model::UserWalletAccountRecord},
};

const GET_USER_WALLET_ACCOUNT_BY_USER_ID: &str = r#"
    SELECT
        user_id,
        wallet_address,
        network,
        created_at
    FROM wallet_accounts
    WHERE user_id = $1
      AND wallet_address IS NOT NULL
"#;

pub async fn get_user_wallet_account_by_user_id(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<UserWalletAccountRecord>, AuthError> {
    sqlx::query_as::<_, UserWalletAccountRecord>(GET_USER_WALLET_ACCOUNT_BY_USER_ID)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AuthError::from)
}
