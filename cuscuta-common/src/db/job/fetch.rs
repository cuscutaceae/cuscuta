use redis::{Client, TypedCommands};
use serde::Deserialize;

use crate::db::{job::JobEssential, redis::Error};

/// 不保证同步性，获取任意List中的JSON内容
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
pub fn fetch_result<T>(redis_client: &Client, list_key: &str) -> Result<Vec<T>, Error>
where
    T: for<'a> Deserialize<'a>,
{
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let range = connection.lrange(list_key, 0, -1).map_err(Error::Redis)?;
    let mut out = Vec::new();
    for it in &range {
        out.push(serde_json::from_str::<T>(it).map_err(|e| {
            Error::BadData(format!("failed to deserialize string to json: {it}({e})"))
        })?);
    }
    Ok(out)
}

/// [`search_position`]的返回结果
#[derive(Debug, PartialEq, Eq)]
pub enum SearchPositionResult {
    /// 任务正在进行
    Pending,

    /// 任务正在排队，且在搜索范围内
    QueueingFound(usize),

    /// 任务正在排队，但超出搜索范围外
    QueueingNotFound,
}

/// 在任务队列中搜索Job的位置，用来计算任务剩余时间\
/// 由于函数的特殊性和非原子性（原子化没必要且易阻塞），本函数返回的结果可能有偏差
///
/// 这个函数的开销预计比较大
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
pub fn search_position(
    redis_client: &Client,
    limit: usize,
    job_uid: &str,
    sub_queue_name: &str,
) -> Result<SearchPositionResult, Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let pending_result = connection
        .xpending_count(sub_queue_name, "default_group", "-", "+", 50)
        .map_err(Error::Redis)?;
    // 试图从PEL解析出真实数据的操作若失败，则忽略；因为这个函数并不要求总是成功
    if pending_result
        .ids
        .into_iter()
        .filter_map(|it| {
            connection
                .xrange(sub_queue_name, it.id.clone(), it.id)
                .map_or(None, |it| it.ids.first().cloned())
        })
        .filter_map(|it| JobEssential::try_from(&it.map).ok())
        .any(|it| it.job_uid == job_uid)
    {
        return Ok(SearchPositionResult::Pending);
    }
    let default_group_info = connection
        .xinfo_groups(sub_queue_name)
        .map_err(Error::Redis)?
        .groups
        .into_iter()
        .find(|it| &it.name == "default_group")
        .ok_or(Error::BadData("no default_group found".to_string()))?;
    let range_result = connection
        .xrange_count(
            sub_queue_name,
            format!("({}", default_group_info.last_delivered_id),
            "+",
            limit,
        )
        .map_err(Error::Redis)?
        .ids
        .into_iter()
        .filter_map(|it| JobEssential::try_from(&it.map).ok())
        .enumerate()
        .find(|(_, id)| id.job_uid == job_uid)
        .map(|it| it.0);
    range_result.map_or_else(
        || Ok(SearchPositionResult::QueueingNotFound),
        |result| Ok(SearchPositionResult::QueueingFound(result)),
    )
}
