use cuscuta_common::{
    api::{
        auto_chilo_xxxxxx::{api_compose_aggregate, api_get_notification, api_login},
        xxxxxx::XxxxxxUrl,
    },
    data::AppVersionData,
    db::account::AccountRow,
};
use sqlx::{Postgres, Transaction};

use cuscuta_common::api::{self, xxxxxx::LoginResult};

pub async fn perform_login(
    xxxxxx_url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    account_row: &AccountRow,
) -> Result<LoginResult, api::Error> {
    tracing::info!("login_interface: email: {}", account_row.account_email);
    tracing::info!("login_interface: pw: {}", account_row.account_password);
    let email = &account_row.account_email;
    tracing::info!("login_interface: performing login api call");
    let result = api_login(
        xxxxxx_url,
        bundle_data,
        email,
        &account_row.account_password,
    )
    .await?;
    tracing::info!("login_interface: account token: {}", result.access_token);
    tracing::info!("login_interface: user id: {}", result.user_id);
    let user_id = &result.user_id.to_string();
    let token = &result.access_token;
    tracing::info!("login_interface: performing aggregate api call");
    api_compose_aggregate(xxxxxx_url, bundle_data, email, user_id, token).await?;
    tracing::info!("login_interface: performing notification api call");
    api_get_notification(xxxxxx_url, bundle_data, email, user_id, token).await?;
    Ok(result)
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
            auto_chilo_xxxxxx::api_list_friend,
            xxxxxx::{FriendListResult1, XxxxxxUrl},
        },
        data::AppVersionData,
        db::{self, account::AccountRow},
    };
    use reqwest::StatusCode;

    use crate::{
        api_compat::xxxxxx_safe_call_ex_worker,
        db::{
            account::{perform_login, update_account_info},
            postgresql::try_open_transaction,
        },
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
        xxxxxx_url: &XxxxxxUrl,

        bundle_data: &AppVersionData,
        account_row: &AccountRow,
        force_login: bool,
    ) -> Result<TokenUpdateResult, Error> {
        //FIXME refactor this
        let current_row =
            if account_row.temp_token.is_none() || account_row.user_id.is_none() || force_login {
                let login_result = perform_login(xxxxxx_url, bundle_data, account_row)
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
        tracing::info!(
            "check_token: fetch friends: {token}, {user_id}, {}, {bundle_data:?}",
            current_row.account_email
        );
        let user_id = user_id.to_string();
        Ok((
            current_row.clone(),
            xxxxxx_safe_call_ex_worker(
                |status| status != StatusCode::TOO_MANY_REQUESTS,
                || {
                    api_list_friend(
                        xxxxxx_url,
                        bundle_data,
                        &current_row.account_email,
                        &user_id,
                        &token,
                    )
                },
            )
            .await
            .map_err(Error::Api)?,
        )
            .into())
    }
}
