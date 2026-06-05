use std::ops::Range;

use axum::{Form, Json, response::IntoResponse};
use base64::Engine;
use chrono::Utc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    data::{CONFIG, SONG_LIST},
    db::{account::count_active_account, postgresql::try_open_transaction, redis::REDIS_CLIENT},
    endpoints::{Error, ErrorType},
};

use cuscuta_common::{
    db::job::{JobEssential, scan_sub_queue, write_job},
    quick_fetch::QuickFetch,
};

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum EnqueueResult {
    Success {
        success: bool,
        query_token: String,
    },
    Failed {
        success: bool,
        code: i64,
        message: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct EnqueueBody {
    friend_code: String,
}

/// 注意，由于SCAN不保证一致性，故此操作并不幂等，请勿简单扩增entry实例
pub async fn enqueue(Form(form): Form<EnqueueBody>) -> impl IntoResponse {
    async fn op(form: EnqueueBody) -> anyhow::Result<String, Error> {
        let config = CONFIG
            .try_read(std::clone::Clone::clone)
            .map_err(|_| Error::NotReady(ErrorType::ConfigNotReady))?;
        let redis_client = REDIS_CLIENT
            .get()
            .ok_or(Error::NotReady(ErrorType::RedisNotReady))?;
        let transaction = try_open_transaction()
            .await
            .map_err(|e| Error::DbExtend(ErrorType::FailedTransactionOpenDb, e))?;
        let song_list_len = SONG_LIST
            .try_read(|it| {
                it.iter()
                    .map(|song| song.difficulties.len())
                    .collect::<Vec<_>>()
            })
            .map_err(|_| Error::NotReady(ErrorType::SongListNotReady))?;
        let active_account_count = count_active_account(transaction)
            .await
            .map_err(|e| Error::Db(ErrorType::FailedCountDb, e))?;
        if active_account_count == 0 {
            return Err(Error::NoWorker(ErrorType::NoWorker));
        }
        let ranges = split_weighted_ranges(&song_list_len, active_account_count);
        let queues = scan_sub_queue(redis_client)
            .map_err(|e| Error::RedisExtend(ErrorType::FailedScanRedis, e))?;
        let timestamp = Utc::now().timestamp().to_string();
        let mut job_ids = Vec::new();
        for range in ranges {
            let target_queue = queues.iter().find(|q| q.segment == range);
            let (queue_name, exist) = match target_queue {
                Some(it) => (it.name.clone(), true),
                //TODO add hash here
                None => (
                    format!(
                        "cuscuta:jobs:chunk_{}_{}_{}_{}",
                        "00000000", timestamp, range.start, range.end
                    ),
                    false,
                ),
            };
            #[allow(clippy::cast_possible_truncation)]
            let job_essential = JobEssential::new(
                form.friend_code.clone(),
                timestamp.clone(),
                range.start.cast_signed() as i32,
                range.len().cast_signed() as i32,
                0,
            );
            let job_id = write_job(
                redis_client,
                &job_essential,
                queue_name,
                !exist,
                config.redis_stream_refresh_ttl,
            )
            .map_err(|e| Error::Redis(ErrorType::FailedEnqueueRedis, e))?;
            job_ids.push(format!("{}_{}", job_id, job_essential.job_uid));
        }
        let query_argument = base64::prelude::BASE64_URL_SAFE.encode(format!(
            "{}-{}|{}",
            form.friend_code.clone(),
            timestamp,
            job_ids.join(",")
        ));
        Ok(query_argument)
    }
    match op(form).await {
        Ok(token) => (
            StatusCode::OK,
            Json(EnqueueResult::Success {
                success: true,
                query_token: token,
            }),
        ),
        Err(e) => {
            log::warn!("endpoint enqueue failed: {e}");
            (
                e.get_status_code(),
                Json(EnqueueResult::Failed {
                    success: false,
                    code: e.get_error_type() as i64,
                    message: format!("{e}"),
                }),
            )
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn split_weighted_ranges(arr: &[usize], count: usize) -> Vec<Range<usize>> {
    let total_weights = arr.iter().sum::<usize>() as f64;
    let avg_weights = total_weights / (count - 1) as f64;
    let mut ws = 0f64;
    let mut last_i = 0usize;
    let mut ranges = Vec::new();
    for (i, it) in arr.iter().enumerate() {
        let current_weight = *it as f64;
        ws += current_weight;
        if ws >= avg_weights {
            ranges.push(last_i..i);
            last_i = i;
            ws -= avg_weights;
        }
    }
    if last_i < arr.len() {
        ranges.push(last_i..arr.len());
    }
    ranges
}
