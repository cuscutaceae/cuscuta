pub mod account;

pub mod redis {
    use std::sync::OnceLock;
    pub static REDIS_CLIENT: OnceLock<redis::Client> = OnceLock::new();
}

pub mod postgresql {
    use cuscuta_common::db::postgresql::Error;
    use sqlx::{Postgres, Transaction};
    use std::sync::OnceLock;
    use tokio::sync::RwLock;
    pub static POSTGRESQL_POOL: OnceLock<RwLock<Option<sqlx::PgPool>>> = OnceLock::new();

    pub async fn try_open_transaction<'a>() -> Result<Transaction<'a, Postgres>, Error> {
        cuscuta_common::db::postgresql::try_open_transaction(&POSTGRESQL_POOL).await
    }
}
