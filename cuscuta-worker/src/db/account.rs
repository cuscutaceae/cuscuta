use std::{env, str::FromStr};

use chrono::Local;
use cuscuta_common::{data::BundleData, db::account::AccountRow};
use reqwest::{StatusCode, Url};
use sqlx::{Postgres, Transaction};

use cuscuta_common::api::{
    self,
    chilo::chilo_generate,
    xxxxxx::{LoginResult, api_login},
};

pub async fn perform_login(
    bundle_data: &BundleData,
    account_row: &AccountRow,
) -> Result<LoginResult, api::Error> {
    let url =
        Url::from_str(&env::var("API_LOGIN").map_err(|e| api::Error::Env(e, "API_LOGIN".into()))?)
            .unwrap_or_else(|_| Url::from_str("http://nofyso:11451/auth/login").unwrap());
    let timestamp = Local::now().timestamp_millis().to_string();
    let random_challenge = match chilo_generate(
        &timestamp,
        &format!("{}{}", "grant_type=client_credentials", url.path()),
        "login",
    )
    .await?
    {
        api::chilo::ChiloResult::Success { value, .. } => value,
        api::chilo::ChiloResult::Failed { message, .. } => {
            log::warn!("login_interface: failed to generate challenge: {message}");
            return Err(api::Error::BadStatus(
                StatusCode::INTERNAL_SERVER_ERROR,
                message,
            ));
        }
    };
    log::info!("login_interface: challenge: {random_challenge}");
    log::info!("login_interface: email: {}", account_row.account_email);
    log::info!("login_interface: pw: {}", account_row.account_password);
    api_login(
        bundle_data,
        &account_row.account_email,
        &account_row.account_password,
        &random_challenge,
    )
    .await
}

pub async fn update_account_info(
    mut tx: Transaction<'_, Postgres>,
    account_row: &AccountRow,
    login_result: &LoginResult,
) -> Result<AccountRow, sqlx::Error> {
    let row: AccountRow = sqlx::query_as(
        r"
        UPDATE account_table
        SET user_id = $2, temp_token = $3
        WHERE id = $1
        RETURNING id, account_email, account_password, user_id, temp_token, state, rate, lease_time;
        ",
    )
    .bind(account_row.id)
    .bind(login_result.user_id)
    .bind(login_result.access_token.clone())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row)
}

pub mod auto {
    use cuscuta_common::{
        api::{
            self,
            xxxxxx::{FriendListResult1, api_list_friend},
        },
        data::BundleData,
        db::{self, account::AccountRow},
    };

    use crate::db::{
        account::{perform_login, update_account_info},
        postgresql::try_open_transaction,
    };

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("api error: {0}")]
        Api(api::Error),
        #[error("db error: {0}")]
        Db(db::postgresql::Error),
    }

    pub struct TokenUpdateResult {
        pub account_row: AccountRow,
        pub friends: FriendListResult1,
    }

    impl From<(AccountRow, FriendListResult1)> for TokenUpdateResult {
        fn from(value: (AccountRow, FriendListResult1)) -> Self {
            Self {
                account_row: value.0,
                friends: value.1,
            }
        }
    }

    /// Warn: God function
    pub async fn check_and_update_token(
        bundle_data: &BundleData,
        account_row: &AccountRow,
        force_login: bool,
    ) -> Result<TokenUpdateResult, Error> {
        //FIXME refactor this
        let current_row =
            if account_row.temp_token.is_none() || account_row.user_id.is_none() || force_login {
                let login_result = perform_login(bundle_data, account_row)
                    .await
                    .map_err(Error::Api)?;
                let transaction = try_open_transaction().await.map_err(Error::Db)?;
                update_account_info(transaction, account_row, &login_result)
                    .await
                    .map_err(db::postgresql::Error::Sql)
                    .map_err(Error::Db)?;
                &AccountRow {
                    temp_token: login_result.access_token.into(),
                    user_id: login_result.user_id.into(),
                    ..account_row.clone()
                }
            } else {
                account_row
            };
        let user_id = current_row.user_id.expect("this should not happen #1");
        let token = current_row
            .temp_token
            .clone()
            .expect("this should not happen #2");
        log::info!(
            "check_token: fetch friends: {token}, {user_id}, {}, {bundle_data:?}",
            current_row.account_email
        );
        Ok((
            current_row.clone(),
            api_list_friend(
                bundle_data,
                &current_row.account_email,
                &user_id.to_string(),
                &token,
            )
            .await
            .map_err(Error::Api)?,
        )
            .into())
    }
}
