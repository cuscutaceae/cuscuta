/// 事件相关
pub mod event;

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

/// Worker事件
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerEvent {
    /// 事件的类型
    pub event_type: WorkerEventType,

    /// 问题所发生的Worker的id
    pub worker_id: String,

    /// 发生时的时间戳
    pub timestamp: i64,

    /// 信息
    pub message: String,
}

/// Worker事件种类
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[repr(u8)]
pub enum WorkerEventType {
    /// 最严重的错误（例如Worker离线）
    Fatal = 9,

    /// Worker一般信息
    Info = 1,

    /// Worker痕迹信息（例如拉取到任务，任务完成——会出现很多）
    Trace = 0,

    /// Worker警告（例如工作循环失败）
    Warn = 2,
}
