use std::{collections::HashMap, ops::Range};

use chrono::Utc;
use redis::{Client, FromRedisValue, ScanOptions, TypedCommands, streams::StreamId};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{
    api::xxxxxx::SongScore,
    db::redis::{Error, job_index_redis_key, job_output_index_redis_key, job_sub_queue_redis_key},
};

/// 代表一个任务分片，对应Redis数据库中的分任务队列
///
/// 一个任务队列的Key格式如下：\
/// `cuscuta:jobs:chunk_hash_timestamp_from_to`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubQueue {
    /// 这个任务队列的全名，例如`cuscuta:jobs:chunk_00000000_123456789_114_514`
    pub name: String,

    /// \[WIP\] 这个任务队列所对应的`song_list`的hash，目前处于未完成状态
    pub hash: String,

    /// 这个任务队列创建时的时间戳，对应任务队列名字的`timestamp`
    pub timestamp: u64,

    /// 这个任务队列所占有的分块，对应队列名字的`from`和`to`
    pub segment: Range<usize>,
}

impl SubQueue {
    /// 获取任务的后缀
    #[must_use]
    pub fn get_postfix(&self) -> String {
        format!(
            "chunk_{}_{}_{}_{}",
            self.hash, self.timestamp, self.segment.start, self.segment.end
        )
    }
}

impl TryFrom<&str> for SubQueue {
    type Error = Error;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        let chunk_info: Vec<&str> = name.split('_').collect();
        if chunk_info.len() != 5 {
            return Err(Error::BadData(format!("bad queue name: {name}")));
        }
        let segment_from = chunk_info[3].parse::<usize>().map_err(|e| {
            Error::BadData(format!(
                "bad segment start \"{}\" of {} ({e})",
                chunk_info[3], name
            ))
        })?;
        let segment_to = chunk_info[4].parse::<usize>().map_err(|e| {
            Error::BadData(format!(
                "bad segment end \"{}\" of {} ({e})",
                chunk_info[4], name
            ))
        })?;
        if segment_from > segment_to {
            return Err(Error::BadData(format!("bad segment (end<start): {name}")));
        }
        Ok(Self {
            name: name.to_string(),
            hash: chunk_info[1].to_string(),
            timestamp: chunk_info[2].parse::<u64>().map_err(|e| {
                Error::BadData(format!(
                    "bad timestamp \"{}\" of {} ({e})",
                    chunk_info[2], name
                ))
            })?,
            segment: segment_from..segment_to,
        })
    }
}

/// 一个Worker负责的`Job`实例，包含`Job`的关键信息和临时状态信息
#[derive(Debug, Clone)]
pub struct Job {
    // From Redis
    /// 任务在Redis队列（Stream）中的id
    pub job_id: String,

    /// 任务的关键信息
    pub essential: JobEssential,

    // States
    /// 源任务队列
    pub sub_queue: SubQueue,

    /// Job的内部状态
    pub state: JobState,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.essential == other.essential
    }
}

impl Job {
    /// 获取Job对应的结果类队列id
    #[must_use]
    pub fn get_stream_key_postfix(&self) -> String {
        self.essential.get_stream_key_postfix()
    }
}

/// 记录任务的状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// 任务刚刚被拉取，还没有加好友
    Pulled {
        /// 任务开始时的时间戳
        start_timestamp: i64,
    },

    /// 任务已加好友，正在进行
    Pending {
        /// 任务的好友ID
        friend_user_id: String,

        /// 任务目前进行的长度
        current_length: usize,

        /// 任务开始时的时间戳
        start_timestamp: i64,
    },

    /// 任务已经完成，等待清理
    Finished {
        /// 任务的好友ID
        friend_user_id: String,

        /// 任务开始时的时间戳
        start_timestamp: i64,
    },

    /// 任务已被清理
    Cleaned,
}

/// 一个任务的index信息，会被存放至`cuscuta:result:index:...`和`cuscuta:pending:index:...`中
#[derive(Debug, Serialize, Deserialize)]
pub struct JobTag {
    /// 这个任务最后一次成功完成时在Redis队列中的id
    pub job_last_id: String,

    /// 这个任务对应的队列来源
    pub queue: SubQueue,

    /// 这个任务的关键信息
    pub job_essential: JobEssential,
}

impl TryFrom<(SubQueue, StreamId)> for Job {
    type Error = Error;
    fn try_from((sub_queue, id): (SubQueue, StreamId)) -> Result<Self, Self::Error> {
        let map = id.map;
        let timestamp = Utc::now().timestamp_millis();
        Ok(Self {
            job_id: id.id,
            essential: JobEssential::try_from(&map)?,
            sub_queue,
            state: JobState::Pulled {
                start_timestamp: timestamp,
            },
        })
    }
}

/// `Job`的关键信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct JobEssential {
    /// 查询对象的好友代码
    pub friend_code: String,

    /// 任务入列时的时间戳
    pub timestamp: String,

    /// 任务开始时的游标
    pub cursor_start: i32,

    /// 任务需要轮询的曲目长度
    pub cursor_length: i32,

    /// \[WIP\] 任务的重试次数，正在实现
    pub retry_count: i32,

    /// 任务的唯一ID，不应随任务重新入队而改变
    pub job_uid: String,
}

impl JobEssential {
    /// 创建新任务重要信息
    ///
    /// 提供任务基本参数，生成新的`JobEssential`\
    /// 其中`job_uid`由这个函数自动使用SHA256生成
    #[must_use]
    pub fn new(
        friend_code: String,
        timestamp: String,
        cursor_start: i32,
        cursor_length: i32,
        retry_count: i32,
    ) -> Self {
        let traits: Vec<_> = friend_code
            .as_bytes()
            .iter()
            .chain(timestamp.as_bytes().iter())
            .chain(cursor_start.to_le_bytes().iter())
            .chain(cursor_length.to_le_bytes().iter())
            .copied()
            .collect();
        let digest = hex::encode(sha2::Sha256::digest(traits));
        Self {
            friend_code,
            timestamp,
            cursor_start,
            cursor_length,
            retry_count,
            job_uid: digest,
        }
    }

    /// 获取Job对应的结果类队列id
    #[must_use]
    pub fn get_stream_key_postfix(&self) -> String {
        format!("{}-{}", self.friend_code.clone(), self.timestamp.clone())
    }
}

impl TryFrom<&HashMap<String, redis::Value>> for JobEssential {
    type Error = Error;

    fn try_from(map: &HashMap<String, redis::Value>) -> Result<Self, Self::Error> {
        Ok(Self::new(
            from_redis(map, "job:friend_code")?,
            from_redis(map, "job:timestamp")?,
            from_redis(map, "job:cursor_start")?,
            from_redis(map, "job:cursor_length")?,
            from_redis(map, "job:retry_count")?,
        ))
    }
}

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

/// 将JobTag内容写入index，以便生成token供查询
///
/// # Errors
/// 本函数的错误全部来自[`redis::RedisError`]
pub fn write_job_index(redis_client: &Client, job_tag: &JobTag) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    connection
        .lpush(
            job_index_redis_key(&job_tag.job_essential.get_stream_key_postfix()),
            serde_json::to_string(job_tag).map_err(|e| {
                Error::BadData(format!("failed to serialize JobTag: {job_tag:?} :{e}"))
            })?,
        )
        .map_err(Error::Redis)?;
    Ok(())
}

/// 不保证同步性，搜索工作队列分片
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误，以及读取内容不符合预期的错误，参见[`crate::db::job::Error`]
pub fn scan_sub_queue(redis_client: &Client) -> Result<Vec<SubQueue>, Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let mut sub_queues = Vec::new();
    for it in connection
        .scan_options::<String>(
            ScanOptions::default()
                .with_count(100)
                .with_pattern("cuscuta:jobs:*")
                .with_type("stream"),
        )
        .map_err(Error::Redis)?
    {
        let name = it.map_err(Error::Redis)?;
        sub_queues.push(SubQueue::try_from(name.as_str())?);
    }
    sub_queues.sort_by_key(|it| it.timestamp);
    Ok(sub_queues)
}

/// 不保证同步性，搜索任务的index
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
/// 这个函数被用于辅助确认任务完成情况
///
/// # Errors
/// 这个函数产生的错误来自Redis的错误[`redis::RedisError`]，以及读取内容不符合预期的错误
pub fn fetch_pending_tags(redis_client: &Client, postfix: &str) -> Result<Vec<JobTag>, Error> {
    fetch_job_tags(redis_client, &job_index_redis_key(postfix))
}

fn fetch_job_tags(redis_client: &Client, key: &str) -> Result<Vec<JobTag>, Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let range = connection.lrange(key, 0, -1).map_err(Error::Redis)?;
    let mut out = Vec::<JobTag>::new();
    for it in &range {
        out.push(serde_json::from_str(it).map_err(|e| {
            Error::BadData(format!("failed to deserialize string to json: {it}({e})"))
        })?);
    }
    Ok(out)
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
        .xpending_count(sub_queue_name, "default_group", "-", "+", "")
        .map_err(Error::Redis)?;
    // 试图从PEL解析出真实数据的操作若失败，则忽略；因为这个函数并不要求总是成功
    if pending_result
        .ids
        .into_iter()
        .filter_map(|it| {
            connection
                .xrange(sub_queue_name, it.id.clone(), it.id)
                .map(|it| it.ids.first().cloned())
                .unwrap_or(None)
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

fn from_redis<T>(value: &HashMap<String, redis::Value>, key: &str) -> Result<T, Error>
where
    T: FromRedisValue,
{
    T::from_redis_value(
        value
            .get(key)
            .ok_or(Error::BadData(format!("no key found {key}")))?
            .clone(),
    )
    .map_err(|e| Error::BadData(format!("failed to parse redis value: {key} : {e}")))
}
