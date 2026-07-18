use chrono::Utc;
use redis::Client;

use crate::db::{
    log::{WorkerEvent, WorkerEventType},
    redis::{Error, worker_event_redis_key},
};

/// 向事件列表写入新事件
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
///
pub fn write_event(
    redis_client: &Client,
    event_type: WorkerEventType,
    worker_id: String,
    message: String,
) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let mut pipe = redis::pipe();
    pipe.lpush(
        worker_event_redis_key(),
        serde_json::to_string(&WorkerEvent {
            event_type,
            worker_id,
            timestamp: Utc::now().timestamp_millis(),
            message,
        })
        .map_err(|e| Error::BadData(format!("failed to parse struct to json: ({e})")))?,
    );
    // TODO: hard coded limit
    pipe.ltrim(worker_event_redis_key(), 0, 100_000);
    pipe.exec(&mut connection).map_err(Error::Redis)?;
    Ok(())
}

/// 使用`REDIS_CLIENT`尝试向事件列表写入新事件
///
/// 很不卫生对吧，但是很好用
#[macro_export]
macro_rules! try_write_event {
    ($redis_client:expr, $worker_id:expr, $event_type:expr, $message:expr) => {{
        use cuscuta_common::db::log::event::write_event;
        use log;
        if let Some(redis_client) = $redis_client.get()
            && let Some(worker_id) = $worker_id.get()
            && let Err(ie) = write_event(redis_client, $event_type, worker_id.to_string(), $message)
        {
            log::warn!("failed to write event: {ie}");
        }
    }};
}
