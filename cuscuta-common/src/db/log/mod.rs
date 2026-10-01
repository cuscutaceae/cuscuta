/// 状态相关
pub mod status;

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::db::job::{Job, SubQueue};

/// 记录Worker工作状态
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerStatus<'a> {
    /// 最后一次活跃的时间戳
    pub last_active_timestamp: i64,

    /// 当前工作指针
    pub cursor: usize,

    /// 当前工作分片
    pub sub_queue: Option<Cow<'a, SubQueue>>,

    /// 当前Job
    pub jobs: Cow<'a, [Job]>,
}
