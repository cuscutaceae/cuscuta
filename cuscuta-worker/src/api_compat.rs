use cuscuta_common::api::{
    self,
    xxxxxx::auto::{xxxxxx_safe_call, xxxxxx_safe_call_ex},
};
use reqwest::StatusCode;

use crate::data::Config;

#[allow(clippy::cast_possible_truncation)]
pub async fn xxxxxx_safe_call_ex_worker<'a, F, R, T, Fut>(
    config: &Config,
    fail_cond: T,
    f: F,
) -> Result<R, api::Error>
where
    Fut: Future<Output = Result<R, api::Error>> + 'a + Send,
    R: Send + 'a,
    F: Fn() -> Fut,
    T: Fn(StatusCode) -> bool,
{
    xxxxxx_safe_call_ex(
        config.worker_max_retry_count,
        config.worker_exponential_backoff_base_millis,
        config.worker_exponential_backoff_multiplier,
        config.worker_exponential_backoff_max_delay_millis,
        fail_cond,
        f,
    )
    .await
}

#[allow(clippy::cast_possible_truncation)]
pub async fn xxxxxx_safe_call_worker<'a, F, R, Fut>(config: &Config, f: F) -> Result<R, api::Error>
where
    Fut: Future<Output = Result<R, api::Error>> + 'a + Send,
    R: Send + 'a,
    F: Fn() -> Fut,
{
    xxxxxx_safe_call(
        config.worker_max_retry_count,
        config.worker_exponential_backoff_base_millis,
        config.worker_exponential_backoff_multiplier,
        config.worker_exponential_backoff_max_delay_millis,
        f,
    )
    .await
}
