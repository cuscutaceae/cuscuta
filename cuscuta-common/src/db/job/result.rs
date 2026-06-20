use redis::{Client, TypedCommands};

use crate::{
    api::xxxxxx::SongScore,
    db::{
        job::{JobEssential, JobTag, fetch_job_tags},
        redis::{Error, job_index_redis_key, job_output_index_redis_key},
    },
};

/// 不保证同步性，搜索任务的完成index
///
/// 这个函数被用于确认任务完成情况
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
pub fn fetch_result_tags(redis_client: &Client, postfix: &str) -> Result<Vec<JobTag>, Error> {
    fetch_job_tags(redis_client, &job_output_index_redis_key(postfix))
}

/// 不保证同步性，搜索任务的index
///
/// 这个函数被用于确认任务的预期结果
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
pub fn fetch_pending_tags(redis_client: &Client, postfix: &str) -> Result<Vec<JobTag>, Error> {
    fetch_job_tags(redis_client, &job_index_redis_key(postfix))
}

/// 不保证同步性，获取任务的执行结果
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
pub fn fetch_result(redis_client: &Client, job_key: &str) -> Result<Vec<SongScore>, Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let range = connection
        .lrange(format!("cuscuta:results:value:{job_key}"), 0, -1)
        .map_err(Error::Redis)?;
    let mut out = Vec::<SongScore>::new();
    for it in &range {
        out.push(serde_json::from_str(it).map_err(|e| {
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
