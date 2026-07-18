use std::borrow::Cow;

use chrono::Utc;
use redis::Client;

use crate::db::{
    job::{Job, SubQueue},
    log::WorkerStatus,
    redis::{Error, worker_status_redis_key},
};

/// 更新Worker状态
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
///
pub fn update_worker_status(
    redis_client: &Client,
    worker_id: &str,
    cursor: usize,
    sub_queue: Option<&SubQueue>,
    jobs: &[Job],
) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let mut pipe = redis::pipe();
    pipe.set(
        worker_status_redis_key(worker_id),
        serde_json::to_string(&WorkerStatus {
            last_active_timestamp: Utc::now().timestamp_millis(),
            cursor,
            sub_queue: sub_queue.map(Cow::Borrowed),
            jobs: Cow::Borrowed(jobs),
        })
        .map_err(|e| Error::BadData(format!("failed to parse struct to json: ({e})")))?,
    );
    // TODO: hard coded expire time
    pipe.expire(worker_status_redis_key(worker_id), 60);
    pipe.exec(&mut connection).map_err(Error::Redis)?;
    Ok(())
}
