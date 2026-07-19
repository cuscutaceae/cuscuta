use std::collections::HashMap;

use cuscuta_common::{
    data::Song,
    db::{
        self,
        job::{
            Job, JobFailure, JobFailureResuming, JobFailureType, JobState, SubQueue,
            scan_sub_queue,
            track::{JobTrackQueueStatus, JobTrackTag},
        },
        log::WorkerEventType,
    },
};
use redis::{
    Client, TypedCommands,
    streams::{StreamAutoClaimOptions, StreamId, StreamReadOptions},
};

use crate::{
    data::Config,
    worker::{Error, update_job_track_info},
    worker_write_event,
};

// 好吧我承认这里写的有点脏了
pub async fn scan_sub_queue_and_pull_job(
    redis_client: &Client,
    current_jobs: &mut Vec<Job>,
    cursor: &mut usize,
    config: &Config,
    song_list: &[Song],
    random: &str,
) -> Result<Option<SubQueue>, Error> {
    let current_sub_queue = current_jobs.first().map(|it| it.sub_queue.clone());
    let song_list_len = song_list.len();
    let (new_jobs, current_segments) = if let Some(s) = current_sub_queue {
        (
            pull_jobs(
                current_jobs,
                &s,
                config,
                redis_client,
                random,
                song_list_len,
            )?,
            s,
        )
    } else {
        let sub_queues = scan_sub_queue(redis_client).map_err(|e| match e {
            db::redis::Error::Redis(redis_error) => Error::Redis(redis_error),
            db::redis::Error::BadData(e) => Error::BadState(format!("bad data: {e}")),
        })?;
        let Some((jobs, sub_queue)) = discover_sub_queue(
            current_jobs,
            config,
            redis_client,
            random,
            &sub_queues,
            song_list_len,
        )?
        else {
            return Ok(None);
        };
        *cursor = sub_queue.segment.start;
        (Some(jobs), sub_queue)
    };
    if let Some(new_jobs) = new_jobs {
        for it in new_jobs {
            tracing::info!("worker_loop: pulled job: {it:?}");
            worker_write_event!(WorkerEventType::Trace, format!("pulled: {it:?}"));
            update_job_track_info(
                redis_client,
                &it,
                |it| {
                    it.status = JobTrackQueueStatus::Pending;
                },
                || JobTrackTag {
                    status: JobTrackQueueStatus::Queueing,
                    job_ids: vec![it.job_id.clone()],
                    queue: it.sub_queue.clone(),
                    job_essential: it.essential.clone(),
                    failures: vec![JobFailure::new(
                        JobFailureType::TargetKeyNotFound("pulling job".to_owned()),
                        JobFailureResuming::NoOp,
                    )],
                },
            )?;
            current_jobs.push(it);
        }
    }
    Ok(Some(current_segments))
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn claim_redis_jobs(
    connection: &mut redis::Connection,
    key: &str,
    consumer: &str,
    min_idle_time_secs: f64,
    target_counts: usize,
) -> Result<Vec<StreamId>, Error> {
    if target_counts == 0 {
        return Ok(Vec::new());
    }
    Ok(connection
        .xautoclaim_options(
            key,
            "default_group",
            consumer,
            (min_idle_time_secs * 1000.0) as i64,
            "0-0",
            StreamAutoClaimOptions::default().count(target_counts),
        )
        .map_err(Error::Redis)?
        .claimed)
}

fn fetch_redis_jobs(
    connection: &mut redis::Connection,
    key: &str,
    consumer: &str,
    target_counts: usize,
) -> Result<Vec<StreamId>, Error> {
    if target_counts == 0 {
        return Ok(Vec::new());
    }
    Ok(connection
        .xread_options(
            &[&key],
            &[">"],
            &StreamReadOptions::default()
                .group("default_group", consumer)
                .count(target_counts),
        )
        .map_err(Error::Redis)?
        .map_or(Vec::new(), |it| {
            it.keys.into_iter().next().map_or(Vec::new(), |it| it.ids)
        }))
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn pull_jobs(
    jobs: &[Job],
    sub_queue: &SubQueue,
    config: &Config,
    redis_client: &Client,
    pod_uid: &str,
    total_length: usize,
) -> Result<Option<Vec<Job>>, Error> {
    // TODO: 添加无GROUP找不到的错误处理（跳过）
    let valid_jobs = valid_jobs(jobs);
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let max_jobs = config.worker_max_jobs.try_into().expect("wait... what?");
    if valid_jobs >= 1 {
        let _ = connection.expire(sub_queue.name.clone(), config.redis_stream_refresh_ttl);
    }
    if valid_jobs >= max_jobs {
        return Ok(Option::None);
    }
    let divisions = jobs
        .first()
        .map_or(1, |it| {
            (total_length as f64 / it.sub_queue.segment.len() as f64) as i32
        })
        .max(1);
    let min_idle_time =
        (config.worker_job_max_work_time_secs as f64 / f64::from(divisions)).round();
    let claimed_jobs = match claim_redis_jobs(
        &mut connection,
        &sub_queue.name,
        pod_uid,
        min_idle_time,
        max_jobs.saturating_sub(valid_jobs),
    ) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("worker_loop_pull_jobs: failed to claim jobs: {e}");
            Vec::new()
        }
    };
    let valid_jobs = valid_jobs + claimed_jobs.len();
    let fetched_jobs = match fetch_redis_jobs(
        &mut connection,
        &sub_queue.name,
        pod_uid,
        max_jobs.saturating_sub(valid_jobs),
    ) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("worker_loop_pull_jobs: failed to fetch jobs: {e}");
            Vec::new()
        }
    };
    let jobs: Vec<_> = fetched_jobs
        .iter()
        .chain(claimed_jobs.iter())
        .map(|it| {
            (sub_queue.clone(), it.clone())
                .try_into()
                .map_err(Error::JobParse)
        })
        .collect::<Result<Vec<Job>, Error>>()?;
    let mut no_duplicated_jobs = HashMap::new();
    for job in &jobs {
        no_duplicated_jobs.entry(&job.essential).or_insert(job);
    }
    Ok(Some(
        no_duplicated_jobs
            .into_iter()
            .collect::<Vec<(_, _)>>()
            .into_iter()
            .map(|it| it.1.clone())
            .collect(),
    ))
}

fn discover_sub_queue(
    jobs: &[Job],
    config: &Config,
    redis_client: &Client,
    pod_uid: &str,
    sub_queues: &[SubQueue],
    total_length: usize,
) -> Result<Option<(Vec<Job>, SubQueue)>, Error> {
    for queue in sub_queues {
        let Some(jobs) = pull_jobs(jobs, queue, config, redis_client, pod_uid, total_length)?
        else {
            continue;
        };
        if jobs.is_empty() {
            continue;
        }
        return Ok(Some((jobs, queue.clone())));
    }
    Ok(None)
}

fn valid_jobs(jobs: &[Job]) -> usize {
    jobs.iter()
        .filter(|it| !matches!(it.state, JobState::Cleaned))
        .count()
}

#[cfg(test)]
mod test {
    use cuscuta_common::{
        db::job::{Job, JobState},
        test::mock::SimpleMockable,
    };

    use crate::worker::pull::valid_jobs;

    #[test]
    fn valid_jobs_count_test() {
        assert_eq!(
            valid_jobs(&[
                Job {
                    state: JobState::mock_cleaned(),
                    ..Job::mock()
                },
                Job {
                    state: JobState::mock_cleaned(),
                    ..Job::mock()
                },
                Job {
                    state: JobState::mock_cleaned(),
                    ..Job::mock()
                },
                Job {
                    state: JobState::mock_failed(),
                    ..Job::mock()
                },
                Job {
                    state: JobState::mock_finished(),
                    ..Job::mock()
                },
                Job {
                    state: JobState::mock_pending(),
                    ..Job::mock()
                },
                Job {
                    state: JobState::mock_pulled(),
                    ..Job::mock()
                },
            ]),
            4
        );
    }
}
