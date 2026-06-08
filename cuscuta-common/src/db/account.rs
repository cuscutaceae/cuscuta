use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};

/// `PostgreSQL`数据库中账户的数据模型
#[derive(Debug, sqlx::FromRow, Clone)]
pub struct AccountRow {
    /// 数据库条目的id
    pub id: i64,

    /// 账户的电子邮箱（或用户名）
    pub account_email: String,

    /// 账户的密码
    pub account_password: String,

    /// 账户的id，若未登录，则为`None`
    pub user_id: Option<i64>,

    /// 账户的token，若未登录，则为`None`
    pub temp_token: Option<String>,

    /// 账户的状态，可为`Idle`或`Using`
    pub state: String,

    /// 账户的评分
    pub rate: i32,

    /// 账户的租期，\[WIP\] 超过租期的账户会被当作`Idle`状态处理
    pub lease_time: DateTime<Utc>,
}

impl AccountRow {
    /// 同时获取账户的`user_id`和`temp_token`
    #[must_use]
    pub fn check_log_info(&self) -> Option<(i64, String)> {
        let user_id = self.user_id?;
        let token = self.temp_token.clone()?;
        Some((user_id, token))
    }
}

/// 尝试锁定一个账户条目
///
/// 设置账户条目的状态为`Using`并返回自身
///
/// # Errors
/// 这个函数的错误全部来源于sql错误[`sqlx::Error`]
pub async fn try_lock_account(
    mut tx: Transaction<'_, Postgres>,
    worker_account_lease_time_secs: u64,
) -> Result<Option<AccountRow>, sqlx::Error> {
    let current_timestamp = Utc::now().timestamp();
    let picked_id: Option<i64> = sqlx::query_scalar(
        r"
        SELECT id
        FROM account_table
        WHERE rate > $1 AND (state = 'Idle' OR lease_time <= to_timestamp($2))
        ORDER BY id ASC
        FOR UPDATE SKIP LOCKED
        LIMIT 1;
        ",
    )
    .bind(0)
    .bind(current_timestamp)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(id) = picked_id else {
        tx.rollback().await?;
        return Ok(None);
    };
    let row: AccountRow = sqlx::query_as(
        r"
        UPDATE account_table
        SET state = $2, lease_time = $3
        WHERE id = $1
        RETURNING id, account_email, account_password, user_id, temp_token, state, rate, lease_time;
        ",
    )
    .bind(id)
    .bind("Using")
    .bind(Utc::now() + Duration::from_secs(worker_account_lease_time_secs))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row.into())
}

/// 尝试锁定一个账户条目
///
/// 设置状态为`Idle`
///
/// # Errors
/// 这个函数的错误全部来源于sql错误[`sqlx::Error`]
pub async fn try_release_account(
    mut tx: Transaction<'_, Postgres>,
    id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r"
        UPDATE account_table
        SET state = 'Idle'
        WHERE id = $1;
        ",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// 尝试更新一个账户条目的`rate`
///
/// # Errors
/// 这个函数的错误全部来源于sql错误[`sqlx::Error`]
pub async fn update_account_rate(
    mut tx: Transaction<'_, Postgres>,
    account_row: &AccountRow,
    delta: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r"
        UPDATE account_table
        SET rate = rate + ($2)
        WHERE id = $1;
        ",
    )
    .bind(account_row.id)
    .bind(delta)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// 尝试更新一个账户条目的`lease_time`
///
/// # Errors
/// 这个函数的错误全部来源于sql错误[`sqlx::Error`]
pub async fn update_account_lease_time(
    mut tx: Transaction<'_, Postgres>,
    account_row: &AccountRow,
    advanced_seconds: u64,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let current_timestamp = Utc::now().timestamp();
    let time: Option<DateTime<Utc>> = sqlx::query_scalar(
        r"
        UPDATE account_table
        SET lease_time = to_timestamp($2)
        WHERE id = $1
        RETURNING lease_time;
        ",
    )
    .bind(account_row.id)
    .bind(current_timestamp + advanced_seconds.cast_signed())
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(time)
}
