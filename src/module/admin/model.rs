use crate::module::auth::model::{UserRecord, WalletRecord};

#[derive(Debug, Clone)]
pub struct AdminProfile {
    pub user: UserRecord,
    pub wallet: WalletRecord,
}
