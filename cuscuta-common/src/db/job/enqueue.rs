use redis::{Client, TypedCommands};

use crate::db::{job::JobEssential, redis::job_sub_queue_redis_key};

/// 向任务队列写入新任务
///
/// # Errors
/// 本函数的错误全部来自[`redis::RedisError`]
///
/// # Panics
/// 当XADD返回`None`时panic，由于XADD的特性，这个函数理论上永远不会panic
pub fn write_job(
    redis_client: &Client,
    job_essential: &JobEssential,
    postfix: &str,
    recreate_group: bool,
    expire_time: i64,
) -> Result<String, redis::RedisError> {
    let key = job_sub_queue_redis_key(postfix);
    let mut connection = redis_client.get_connection()?;
    if let Err(e) = connection.xgroup_create_mkstream(key.clone(), "default_group", "0-0")
        && !e.to_string().contains("BUSYGROUP")
        && recreate_group
    {
        return Err(e);
    }
    connection.expire(key.clone(), expire_time)?;
    Ok(connection
        .xadd(
            key,
            "*",
            &[
                ("job:friend_code", job_essential.friend_code.clone()),
                ("job:timestamp", job_essential.timestamp.clone()),
                ("job:cursor_start", job_essential.cursor_start.to_string()),
                ("job:cursor_length", job_essential.cursor_length.to_string()),
                ("job:retry_count", job_essential.retry_count.to_string()),
            ],
        )?
        .expect("XADD returns null when no 'NOMKSTREAM' declared, this should not happen"))
}
