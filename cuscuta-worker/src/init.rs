use cuscuta_common::{
    api::{self, xxxxxx::api_delete_friend},
    data::BundleData,
    db::account::{AccountRow, try_lock_account, try_release_account, update_account_rate},
    quick_fetch::QuickFetch,
};
use tokio_util::sync::CancellationToken;

use crate::{
    data::{ACCOUNT_ROW, BUNDLE_DATA, CONFIG},
    db::{
        self,
        account::auto::{TokenUpdateResult, check_and_update_token},
        postgresql::try_open_transaction,
        redis::REDIS_CLIENT,
    },
};

#[derive(Debug, PartialEq, Eq)]
enum Level {
    Retry,
    DirtyAccount,
    Halt,
}
struct Error {
    level: Level,
    description: String,
}
impl From<(Level, String)> for Error {
    fn from(value: (Level, String)) -> Self {
        Self {
            level: value.0,
            description: value.1,
        }
    }
}

async fn get_friend_result(
    bundle_data: &BundleData,
    account_row: &AccountRow,
) -> Result<TokenUpdateResult, Error> {
    let first_result = try_update_token(bundle_data, account_row, false).await;
    match first_result {
        Ok(result) => return Ok(result),
        Err((Level::DirtyAccount, _)) => {}
        Err(e) => return Err(e.into()),
    }
    let second_result = try_update_token(bundle_data, account_row, true).await;
    match second_result {
        Ok(result) => Ok(result),
        Err((Level::DirtyAccount, m)) => Err((Level::Halt, m).into()),
        Err(e) => Err(e.into()),
    }
}

async fn try_init() -> Result<(), Error> {
    tracing::info!("init: cuscuta-worker initializing...");
    let bundle_data = BUNDLE_DATA
        .try_read(std::clone::Clone::clone)
        .map_err(|e| (Level::Retry, format!("failed to read BUNDLE_DATA: {e}")))?;
    let worker_account_lease_time_secs = CONFIG
        .try_read(|it| it.worker_account_lease_time_secs)
        .map_err(|e| (Level::Retry, format!("failed to read BUNDLE_DATA: {e}")))?;
    REDIS_CLIENT
        .get()
        .ok_or_else(|| (Level::Retry, "redis is not ready".to_string()))?
        .get_connection()
        .map_err(|e| {
            (
                Level::Retry,
                format!("failed to connection to redis server: {e}"),
            )
        })?;
    let account_row = try_lock_account(
        try_open_transaction()
            .await
            .map_err(|e| (Level::Retry, format!("failed to open transaction: {e}")))?,
        worker_account_lease_time_secs,
    )
    .await
    .map_err(|e| {
        (
            Level::Retry,
            format!("failed to operate postgresql database: {e}"),
        )
    })?
    .ok_or_else(|| (Level::Halt, "no account found".to_string()))?;
    tracing::info!("init: locked {}", account_row.id);
    ACCOUNT_ROW
        .try_write(|_| Some(account_row.clone()))
        .map_err(|e| (Level::Retry, format!("failed to write ACCOUNT_ROW: {e}")))?;
    let TokenUpdateResult {
        account_row,
        friends,
    } = get_friend_result(&bundle_data, &account_row).await?;
    tracing::info!("init: found {} existing friends", friends.friends.len());
    for friend in friends.friends {
        api_delete_friend(
            &bundle_data,
            &account_row.account_email,
            &account_row
                .user_id
                .ok_or_else(|| (Level::Halt, "unexpected data #1".to_string()))?
                .to_string(),
            &account_row
                .temp_token
                .clone()
                .ok_or_else(|| (Level::Halt, "unexpected data #2".to_string()))?,
            &friend.user_id.to_string(),
        )
        .await
        .map_err(|e| (Level::Halt, format!("failed to clean friends: {e}")))?;
        tracing::info!("init: removed friend: {friend:?}");
    }
    ACCOUNT_ROW
        .try_write(|_| Some(account_row.clone()))
        .map_err(|e| (Level::Retry, format!("failed to write ACCOUNT_ROW: {e}")))?;
    Ok(())
}

async fn try_failed_resume() -> Result<(), Error> {
    let account_row = ACCOUNT_ROW.try_read(std::clone::Clone::clone).ok();
    if let Some(account_row) = account_row {
        let transaction = try_open_transaction()
            .await
            .map_err(|e| (Level::Retry, format!("failed to open transaction: {e}")))?;
        try_release_account(transaction, account_row.id)
            .await
            .map_err(|e| (Level::Retry, format!("failed to release account: {e}")))?;
        ACCOUNT_ROW
            .try_write(|_| None)
            .map_err(|e| (Level::Retry, format!("failed to clear account row: {e}")))?;
    } else {
        tracing::info!("init_resume: no account found, skip");
    }
    Ok(())
}

async fn try_update_token(
    bundle_data: &BundleData,
    account_row: &AccountRow,
    account_dirty: bool,
) -> Result<TokenUpdateResult, (Level, String)> {
    let friends_result = check_and_update_token(bundle_data, account_row, account_dirty).await;
    if let Err(db::account::auto::Error::Api(api::Error::ApiError {
        http_status_code,
        error_code,
        message,
    })) = friends_result
    {
        if http_status_code == 500 {
            return Err((
                Level::Halt,
                format!(
                    "bad hash(HTTP {http_status_code}:{error_code} {message}), is chilo out of dated?"
                ),
            ));
        }
        update_account_rate(
            try_open_transaction()
                .await
                .map_err(|e| (Level::Halt, format!("failed to {e}")))?,
            account_row,
            -1,
        )
        .await
        .map_err(|e| (Level::Retry, format!("failed to update account rate: {e}")))?;
        return Err((
            if account_dirty {
                Level::Halt
            } else {
                Level::DirtyAccount
            },
            format!("failed to login: HTTP: {http_status_code} {error_code}"),
        ));
    }
    match friends_result {
        Ok(result) => Ok(result),
        Err(e) => Err((Level::Retry, format!("failed to fetch friend result: {e}"))),
    }
}

pub async fn cuscuta_init(service_token: &CancellationToken, init_token: &CancellationToken) {
    if let Err(Error { level, description }) = try_init().await {
        if level == Level::Halt {
            tracing::error!("init: serious error occurred in initializing: {description}, halting");
            init_token.cancel();
            service_token.cancel();
            return;
        }
        tracing::warn!("init: failed to initialize temporary, level:{level:?}, desc:{description}");
        tracing::info!("init_resume: resuming states");
        let mut resume_retries = 0;
        loop {
            if let Err(Error { level, description }) = try_failed_resume().await {
                if resume_retries >= 5 || level == Level::Halt {
                    tracing::error!(
                        "init_resume: serious error occurred in initialize resuming: {description}, halting"
                    );
                    init_token.cancel();
                    service_token.cancel();
                    return;
                }
                tracing::warn!(
                    "init_resume: failed to resume temporary, level:{level:?}, desc:{description}"
                );
            } else {
                break;
            }
            resume_retries += 1;
        }
        return;
    }
    tracing::info!("  ┏━╸╻ ╻┏━┓┏━╸╻ ╻╺┳╸┏━┓  ");
    tracing::info!("  ┃  ┃ ┃┗━┓┃  ┃ ┃ ┃ ┣━┫  ");
    tracing::info!("  ┗━╸┗━┛┗━┛┗━╸┗━┛ ╹ ╹ ╹  ");
    tracing::info!("init: let the cuscuta spread...");
    init_token.cancel();
}
