use cuscuta_common::{
    api::{
        self,
        xxxxxx::{SongScore, auto::xxxxxx_safe_call},
    },
    data::{BundleData, Song},
    db::{
        account::AccountRow,
        job::{Job, JobState},
        redis::job_result_value_redis_key,
    },
};
use redis::{Client, TypedCommands};

use crate::{data::Config, worker::Error};

pub fn process_job_with_result(jobs: &mut [Job], scores: &[SongScore]) -> Vec<(String, SongScore)> {
    let mut job_links = Vec::new();
    for job in jobs.iter_mut() {
        let redis_key = job_result_value_redis_key(&job.get_stream_key_postfix());
        let JobState::Pending {
            friend_info,
            current_length,
            start_timestamp,
        } = &mut job.state
        else {
            continue;
        };
        let linked_score = scores.iter().filter(|it| friend_info.user_id == it.user_id);
        *current_length += 1;
        for linked_score in linked_score {
            job_links.push((redis_key.clone(), linked_score.clone()));
        }
        if *current_length >= job.essential.cursor_length.cast_unsigned() as usize {
            job.state = JobState::Finished {
                friend_info: friend_info.clone(),
                start_timestamp: *start_timestamp,
            };
        }
    }
    log::debug!(
        "linked {} results for {:?}",
        job_links.len(),
        scores.first().map(|it| it.song_id.clone())
    );
    job_links
}

pub fn write_result_to_redis(
    redis_client: &Client,
    score_pairs: &[(String, SongScore)],
) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    for (key, score) in score_pairs {
        let json = serde_json::to_string(score)
            .map_err(|e| Error::BadState(format!("failed to serialize data to json: {e}")))?;
        connection.lpush(key, &json).map_err(Error::Redis)?;
    }
    Ok(())
}

pub async fn gather_rank_list<'a>(
    bundle_data: &'a BundleData,
    user_id: &'a str,
    token: &'a str,
    account_row: &'a AccountRow,
    song_list: &'a [Song],
    cursor: usize,
    config: &Config,
) -> Result<Vec<SongScore>, Error> {
    let Some(song) = song_list.get(cursor) else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for difficulty in &song.difficulties {
        let rating_class = difficulty.rating_class.to_string();
        let rank_list = xxxxxx_safe_call(
            config.worker_max_retry_count,
            config.worker_exponential_backoff_base_millis,
            config.worker_exponential_backoff_multiplier,
            config.worker_exponential_backoff_max_delay_millis,
            || {
                api::xxxxxx::api_get_rank_list(
                    bundle_data,
                    &account_row.account_email,
                    user_id,
                    token,
                    &song.id,
                    &rating_class,
                    "0",
                    "11",
                )
            },
        )
        .await
        .map_err(Error::Api)?;
        for it in rank_list {
            result.push(it);
        }
    }
    Ok(result)
}
