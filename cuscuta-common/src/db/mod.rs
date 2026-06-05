/// 账号数据相关
pub mod account;

/// 工作队列相关
pub mod job;

/// 定义了`PostgreSQL`所需要使用的函数和错误
pub mod postgresql {
    use std::sync::OnceLock;

    use sqlx::{Postgres, Transaction};
    use tokio::sync::{RwLock, TryLockError};

    /// 操作`PostgreSQL`时的错误
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        /// 全局变量没有准备好
        #[error("pool is not ready: initialized: {initialized}")]
        NotReady {
            /// 是否已经初始化
            initialized: bool,
        },

        /// `TryLock`操作失败
        #[error("try lock error: {0}")]
        TryLock(TryLockError),

        /// SQL错误
        #[error("sql error: {0}")]
        Sql(sqlx::Error),
    }

    /// 尝试开启一个SQL事务
    ///
    /// # Errors
    /// - 当SQL全局变量未初始化或初始化未完全时，返回[`Error::NotReady`]
    /// - 当全局变量读取失败时，返回[`Error::TryLock`]
    /// - 当出现SQL错误时，返回[`Error::Sql`]
    pub async fn try_open_transaction<'a>(
        postgresql_pool: &OnceLock<RwLock<Option<sqlx::PgPool>>>,
    ) -> Result<Transaction<'a, Postgres>, Error> {
        let sql_pool_option = postgresql_pool
            .get()
            .ok_or(Error::NotReady { initialized: false })?
            .try_read()
            .map_err(Error::TryLock)?;
        let sql_pool = sql_pool_option
            .as_ref()
            .ok_or(Error::NotReady { initialized: true })?;
        let transaction = sql_pool.begin().await.map_err(Error::Sql)?;
        Ok(transaction)
    }
}
