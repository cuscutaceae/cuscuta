use redis::{Client, TypedCommands};
use serde::{Deserialize, Serialize};

use crate::db::{
    job::{Job, JobEssential, JobFailure, SubQueue},
    redis::{Error, job_result_tracking_redis_key},
};

/// 任务的单个完成信息，存放至`cuscuta:status:...`中
#[derive(Debug, Serialize, Deserialize)]
pub struct JobTrackTag {
    /// Job 当前队列状态
    pub status: JobTrackQueueStatus,

    /// 将 `last_job_id` 改为 `job_ids` ，统计使用过的 `job_id` 数目，就可统计出重试次数
    pub job_ids: Vec<String>,

    /// 任务的的分片队列信息
    pub queue: SubQueue,

    /// Job 的关键信息
    pub job_essential: JobEssential,

    /// Job 的失败信息
    pub failures: Vec<JobFailure>,
}

/// 任务的入列情况
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobTrackQueueStatus {
    /// 任务正在排队中，未被认领
    Queueing,

    /// 任务正在执行
    Pending,

    /// 任务遇到了无法恢复的失败
    Failed,

    /// 任务完成
    Success,
}

/// 将 [`JobTrackTag`] 切片写入 index
///
/// # Errors
/// 本函数的错误来自[`redis::RedisError`]（转换到[`Error::Redis`]），以及[`serde_json::Error`]（转换到[`Error::BadData`]）
pub fn batch_write_job_tracking_tag(
    redis_client: &Client,
    job_track_tags: &[JobTrackTag],
) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let mut pipe = redis::pipe();
    for it in job_track_tags {
        pipe.hset(
            job_result_tracking_redis_key(&it.job_essential.get_stream_key_postfix()),
            &it.job_essential.job_uid,
            serde_json::to_string(it).map_err(|e| {
                Error::BadData(format!("failed to serialize JobTrackTag: {it:?} :{e}"))
            })?,
        );
    }
    pipe.exec(&mut connection).map_err(Error::Redis)?;
    Ok(())
}

/// 从 index 读取指定 Job 的 [`JobTrackTag`]
///
/// # Errors
/// 本函数的错误来自[`redis::RedisError`]（转换到[`Error::Redis`]），以及[`serde_json::Error`]（转换到[`Error::BadData`]）
pub fn fetch_job_track_tag(redis_client: &Client, job: &Job) -> Result<JobTrackTag, Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let json_str = connection
        .hget(
            job_result_tracking_redis_key(&job.get_stream_key_postfix()),
            &job.essential.job_uid,
        )
        .map_err(Error::Redis)?
        .ok_or_else(|| {
            Error::BadData(format!(
                "data note found in hash: key:{} field:{}",
                &job.get_stream_key_postfix(),
                job.essential.job_uid
            ))
        })?;
    serde_json::from_str(&json_str).map_err(|e| {
        Error::BadData(format!(
            "failed to serialize JobTrackTag: {json_str:?} :{e}"
        ))
    })
}

/// 从 index 读取所有指定 postfix 的 [`JobTrackTag`]
///
/// # Errors
/// 本函数的错误来自[`redis::RedisError`]（转换到[`Error::Redis`]），以及[`serde_json::Error`]（转换到[`Error::BadData`]）
pub fn fetch_all_job_track_tag(
    redis_client: &Client,
    postfix: &str,
) -> Result<Vec<JobTrackTag>, Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let hashes = connection
        .hgetall(job_result_tracking_redis_key(postfix))
        .map_err(Error::Redis)?;
    hashes
        .into_values()
        .map(|it| {
            serde_json::from_str(&it).map_err(|e| {
                Error::BadData(format!("failed to deserialize string to json: {it}({e})"))
            })
        })
        .collect::<Result<Vec<JobTrackTag>, _>>()
}
