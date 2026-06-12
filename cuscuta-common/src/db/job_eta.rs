use redis::{Client, TypedCommands};

use crate::db::redis::job_eta_redis_key;

/// 与任务剩余时间估算相关的错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Redis错误
    #[error("redis error: {0}")]
    Redis(redis::RedisError),

    /// 任务剩余时间数据不符合预期时的错误
    #[error("bad: {0}")]
    BadData(String),
}

/// 向Redis数据库记录剩余时间
///
/// # Errors
/// 本函数的错误全部来自[`redis::RedisError`]
pub fn record_eta(redis_client: &Client, eta_millis: i64) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    connection
        .lpush(job_eta_redis_key(), eta_millis)
        .map_err(Error::Redis)?;
    Ok(())
}

/// 从Redis数据库计算平均消耗时间，剔除偏差大于3倍标准差的数据
/// 当数据为空时，返回Ok(None)，当剔除后无数据时，使用原始数据的平均值
///
/// # Errors
/// 本函数的错误全部来自[`redis::RedisError`]
#[allow(clippy::cast_precision_loss)]
pub fn fetch_eta(redis_client: &Client, limit: usize) -> Result<Option<f64>, Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let result = connection
        .lrange(
            job_eta_redis_key(),
            0,
            limit.cast_signed().saturating_sub(1),
        )
        .map_err(Error::Redis)?
        .into_iter()
        .map(|it| {
            it.parse::<f64>()
                .map_err(|e| Error::BadData(format!("failed to parse data to u64: {e}")))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    connection
        .ltrim(
            job_eta_redis_key(),
            0,
            limit.cast_signed().saturating_sub(1),
        )
        .map_err(Error::Redis)?;
    if result.is_empty() {
        return Ok(None);
    }
    let avg = result.iter().sum::<f64>() / (result.len() as f64);
    let sd = (result.iter().fold(0f64, |i, it| i + (*it - avg).powi(2))
        / (result.len() as f64 - 1.0))
        .sqrt();
    let filtered: Vec<_> = result
        .iter()
        .filter(|it| (**it - avg).abs() < sd * 3.0)
        .copied()
        .collect();
    if filtered.is_empty() {
        return Ok(Some(avg));
    }
    let filtered_sum = filtered.iter().sum::<f64>();
    Ok(Some(filtered_sum / (filtered.len() as f64)))
}
