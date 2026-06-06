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
            return Err(Error::Internal(ErrorType::InternalNoWorker));
        }
        let ranges = split_weighted_ranges(&song_list_len, active_account_count)?;
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

fn split_weighted_ranges(weights: &[usize], count: usize) -> Result<Vec<Range<usize>>, Error> {
    let t = split_weighted_ranges_min(weights, count)?;
    let iter = weights.iter().copied().enumerate();
    let mut tot = 0;
    let mut last_i = 0;
    let mut out = Vec::new();
    for (i, this) in iter {
        if tot + this > t {
            out.push(last_i..i);
            tot = tot + this - t;
            last_i = i;
        } else {
            tot += this;
        }
    }
    if last_i != weights.len() {
        out.push(last_i..weights.len());
    }
    Ok(out)
}

/// 感谢 Hoyoak 大佬提供的题解！
/// [题解链接](https://www.cnblogs.com/Hoyoak/p/11354580.html)
fn split_weighted_ranges_min(num: &[usize], m: usize) -> Result<usize, Error> {
    fn check(d: usize, num: &[usize], m: usize) -> bool {
        let mut cnt = 0;
        let mut sum = 0;
        for n in num {
            if sum + n <= d {
                sum += n;
            } else {
                sum = *n;
                cnt += 1;
            }
        }
        cnt < m
    }
    let mut l = num
        .iter()
        .max()
        .copied()
        .ok_or(Error::NotReady(ErrorType::SongListNotReady))?;
    let mut r: usize = num.iter().sum();
    while l <= r {
        let mid = (l + r) >> 1;
        if check(mid, num, m) {
            r = mid - 1;
        } else {
            l = mid + 1;
        }
    }
    Ok(l)
}

#[cfg(test)]
mod test {
    use crate::endpoints::enqueue::{split_weighted_ranges, split_weighted_ranges_min};

    #[test]
    fn test_split_weighted_ranges_1() {
        let a = &[4, 2, 4, 5, 1];
        let n = 3;
        println!(
            "{}:{:?}",
            split_weighted_ranges_min(a, n).unwrap(),
            split_weighted_ranges(a, n).unwrap()
        );
    }

    #[test]
    fn test_split_weighted_ranges_2() {
        let a = &[7, 2, 5, 4, 10, 8];
        let n = 3;
        println!(
            "{}:{:?}",
            split_weighted_ranges_min(a, n).unwrap(),
            split_weighted_ranges(a, n).unwrap()
        );
    }

    #[test]
    fn test_split_weighted_ranges_3() {
        let a = &[4, 3, 3, 3, 3, 4, 4, 4, 3, 3, 3, 3, 4, 3];
        let n = 6;
        println!(
            "{}:{:?}",
            split_weighted_ranges_min(a, n).unwrap(),
            split_weighted_ranges(a, n).unwrap()
        );
    }
}
