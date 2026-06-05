use std::time::Duration;

use tokio::time::{self, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

/// 注册一个循环执行的任务，当`token`被取消时，取消循环任务
#[allow(clippy::ignored_unit_patterns)]
pub async fn register_job<F>(token: CancellationToken, secs: u64, f: F)
where
    F: AsyncFn(&CancellationToken),
{
    let mut interval = time::interval(Duration::from_secs(secs));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _=token.cancelled()=>{
                break;
            }
            _=interval.tick()=>{
                f(&token).await;
            }
        }
    }
}

/// 注册一个循环执行的任务，当`service_token`或者`phase_token`被取消时，取消循环任务
#[allow(clippy::ignored_unit_patterns)]
pub async fn register_individual_job<F>(
    service_token: CancellationToken,
    phase_token: CancellationToken,
    secs: u64,
    f: F,
) where
    F: AsyncFn(&CancellationToken, &CancellationToken),
{
    let mut interval = time::interval(Duration::from_secs(secs));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _=phase_token.cancelled()=>{
                break;
            }
            _=service_token.cancelled()=>{
                break;
            }
            _=interval.tick()=>{
                f(&service_token, &phase_token).await;
            }
        }
    }
}
