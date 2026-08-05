use std::{collections::HashMap, ops::Range};

use chrono::Utc;
use redis::{Client, FromRedisValue, ScanOptions, TypedCommands, streams::StreamId};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{api::xxxxxx::FriendInfo, castable_enum_with_arg, db::redis::Error};

/// 剩余时间相关功能
pub mod eta;

/// 任务完成情况相关功能
pub mod fetch;

/// 任务入列相关功能
pub mod enqueue;

/// 任跟踪相关功能
pub mod track;

/// 代表一个任务分片，对应Redis数据库中的分任务队列
///
/// 一个任务队列的Key格式如下：\
/// `cuscuta:jobs:chunk_hash_timestamp_from_to`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// 一个Worker负责的`Job`实例，包含`Job`的关键信息和临时状态信息
#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
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

/// 记录任务的状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobState {
    /// 任务刚刚被拉取，还没有加好友
    Pulled {
        /// 任务开始时的时间戳
        start_timestamp: i64,
    },

    /// 任务已加好友，正在进行
    Pending {
        /// 任务的好友信息
        friend_info: FriendInfo,

        /// 任务目前进行的长度
        current_length: usize,

        /// 任务开始时的时间戳
        start_timestamp: i64,
    },

    /// 任务已经完成，等待清理
    Finished {
        /// 任务的好友信息
        friend_info: FriendInfo,

        /// 任务开始时的时间戳
        start_timestamp: i64,
    },

    /// 任务失败
    Failed {
        /// 任务的好友信息（可能有）
        friend_info: Option<FriendInfo>,

        /// 任务开始时的时间戳
        start_timestamp: i64,

        /// 任务失败情况
        failure_info: JobFailure,
    },

    /// 任务已被清理
    Cleaned,
}

/// 任务的失败情况
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct JobFailure {
    /// Job 失败的原因
    pub fail_type: JobFailureType,

    /// Job 失败的恢复策略
    pub resume_strategy: JobFailureResuming,

    /// Job 失败的时间
    pub timestamp_millis: i64,
}

/// 失败时的恢复策略
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum JobFailureResuming {
    /// 任务直接失败
    Drop,

    /// 无操作
    NoOp,
}

castable_enum_with_arg! {
    /// 任务的失败信息
    #[derive(Debug, Serialize, Deserialize, thiserror::Error, Clone, PartialEq, Eq)]
    #repr(i64)
    pub enum JobFailureType {
        /// 好友找不到，一般是好友码无效
        #[error("friend not found")]
        FriendNotFound = -1,

        /// 无法找到对应的tracking条目，尝试创建新的
        #[error("worker failed to find exist target key: when {0}")]
        TargetKeyNotFound(String) = -2,

        /// Job 重新入队
        #[error("job Reenqueued")]
        Reenqueued = -3,

        /// 远程Api错误
        #[error("remote xxxxxx api error, HTTP {0}: {1:?}")]
        XxxxxxApiError(u16, Option<i64>) = -4,

        /// 其它Api错误
        #[error("other api error: {0:?}")]
        ApiError(String) = -5,
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

impl JobFailure {
    /// 新建一个[`JobTrackFailure`]，自动填写当前时间戳
    #[must_use]
    pub fn new(fail_type: JobFailureType, resume_strategy: JobFailureResuming) -> Self {
        Self {
            fail_type,
            resume_strategy,
            timestamp_millis: Utc::now().timestamp_millis(),
        }
    }
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
        Self {
            friend_code,
            timestamp,
            cursor_start,
            cursor_length,
            retry_count,
            job_uid: "temp".to_owned(),
        }
        .generate_uid()
    }

    /// 从其它字段推断`job_uid`
    #[must_use]
    pub fn generate_uid(self) -> Self {
        let traits: Vec<_> = self
            .friend_code
            .as_bytes()
            .iter()
            .chain(self.timestamp.as_bytes().iter())
            .chain(self.cursor_start.to_le_bytes().iter())
            .chain(self.cursor_length.to_le_bytes().iter())
            .copied()
            .collect();
        Self {
            job_uid: hex::encode(sha2::Sha256::digest(traits)),
            ..self
        }
    }

    /// 获取Job对应的结果类队列id
    #[must_use]
    pub fn get_stream_key_postfix(&self) -> String {
        format!("{}-{}", self.friend_code.clone(), self.timestamp.clone())
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
    sub_queues.sort_by_key(|it| it.segment.start);
    Ok(sub_queues)
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
