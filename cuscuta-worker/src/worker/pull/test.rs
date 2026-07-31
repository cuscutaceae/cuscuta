#![cfg(test)]

use cuscuta_common::{
    data::Song,
    db::{
        job::{Job, JobEssential, SubQueue},
        redis::{job_sub_queue_redis_key, sub_queue_postfix},
    },
};
use cuscuta_test::{
    mock::SimpleMockable,
    mocks::mock::job_state,
    redis::{ClientWrap, fake_enqueue},
    redis_client_image,
};

use crate::{
    data::Config,
    worker::pull::{scan_sub_queue_and_pull_job, valid_jobs},
};

#[tokio::test]
#[allow(clippy::cast_possible_truncation)]
async fn pull_job_test() {
    // 算了还是写一点注释吧我怕以后看不懂了( •̀ ω •́ )✧
    let hash = "0000000";
    let friend_code = "123456789";
    let job_base_timestamp = 1_784_475_024;
    let queue_timestamp = "1784475024";
    let worker_id = "mock_id";
    let mut current_jobs = Vec::new();
    let mut cursor = 0;
    let config = Config::mock();
    let ClientWrap(client, _container) = redis_client_image!();
    let first_jobs_count = config.worker_max_jobs + 2;

    // 入队测试
    for v in 0..first_jobs_count {
        fake_enqueue(
            &client,
            hash,
            friend_code,
            &(job_base_timestamp + v).to_string(),
            queue_timestamp,
        )
        .expect("failed to enqueue fake jobs");
    }

    // 空本地队列首次pull测试
    let sub_queue = scan_sub_queue_and_pull_job(
        &client,
        &mut current_jobs,
        &mut cursor,
        &config,
        &Vec::<Song>::mock(),
        worker_id,
    )
    .await
    .expect("failed to scan sub queue or pull jobs")
    .expect("expected `Some(SubQueue)`");
    assert_eq!(
        sub_queue,
        SubQueue::try_from(
            job_sub_queue_redis_key(&sub_queue_postfix(hash, queue_timestamp, 0, 10)).as_str()
        )
        .expect("failed to parse sub queue, this should not happen")
    );
    assert_eq!(current_jobs.len(), config.worker_max_jobs as usize);
    let last_timestamp = (job_base_timestamp + config.worker_max_jobs - 1).to_string();
    let last_job = current_jobs
        .iter()
        .find(|it| it.essential.timestamp == last_timestamp.as_str())
        .expect("expected `Some(Job)`");
    assert_eq!(
        last_job.essential,
        JobEssential::new(friend_code.to_string(), last_timestamp, 0, 10, 0)
    );
    assert_eq!(last_job.sub_queue, sub_queue);

    // 已有任务再次pull测试
    let append_timestamp = (job_base_timestamp + 100).to_string();
    current_jobs.truncate(1);
    fake_enqueue(
        &client,
        hash,
        friend_code,
        &append_timestamp,
        queue_timestamp,
    )
    .expect("failed to enqueue fake jobs");
    let sub_queue = scan_sub_queue_and_pull_job(
        &client,
        &mut current_jobs,
        &mut cursor,
        &config,
        &Vec::<Song>::mock(),
        worker_id,
    )
    .await
    .expect("failed to scan sub queue or pull jobs")
    .expect("expected `Some(SubQueue)`");
    assert_eq!(
        sub_queue,
        SubQueue::try_from(
            job_sub_queue_redis_key(&sub_queue_postfix(hash, queue_timestamp, 0, 10)).as_str()
        )
        .expect("failed to parse sub queue, this should not happen")
    );
    assert_eq!(
        current_jobs.len(),
        (first_jobs_count - (config.worker_max_jobs - 1) + 1) as usize
    );
    let append_job = current_jobs
        .iter()
        .find(|it| it.essential.timestamp == append_timestamp.as_str())
        .expect("expected `Some(Job)`");
    assert_eq!(
        append_job.essential,
        JobEssential::new(friend_code.to_string(), append_timestamp, 0, 10, 0)
    );
    assert_eq!(append_job.sub_queue, sub_queue);
}

#[test]
fn valid_jobs_count_test() {
    assert_eq!(
        valid_jobs(&[
            Job {
                state: job_state::mock_cleaned(),
                ..Job::mock()
            },
            Job {
                state: job_state::mock_cleaned(),
                ..Job::mock()
            },
            Job {
                state: job_state::mock_cleaned(),
                ..Job::mock()
            },
            Job {
                state: job_state::mock_failed(),
                ..Job::mock()
            },
            Job {
                state: job_state::mock_finished(),
                ..Job::mock()
            },
            Job {
                state: job_state::mock_pending(),
                ..Job::mock()
            },
            Job {
                state: job_state::mock_pulled(),
                ..Job::mock()
            },
        ]),
        4
    );
}
