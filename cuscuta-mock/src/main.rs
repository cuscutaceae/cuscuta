//! cuscuta的mock应用
//!
//! 这只是一个mock，所以实现并不是很严谨……也不是很优雅
//!
//! 出于某些历史原因，它并不依赖`cuscuta-common`

mod data;

use std::{collections::HashMap, env, sync::OnceLock, time::Duration};

use anyhow::anyhow;
use axum::{
    Form, Json, Router,
    extract::Query,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use base64::Engine;
use chrono::{DateTime, Utc};
use cuscuta_common::api::xxxxxx::FriendInfo;
use cuscuta_common::quick_fetch::QuickFetch;
use serde_json::json;
use sha2::Digest;
use tokio::{net::TcpListener, sync::RwLock};

use crate::data::{
    FriendAddForm, FriendModifyResult, FriendRemoveForm, FriendsResult, LoginResult,
    RankListFetchResult, RankListQuery, RankListResult,
};

struct MockAccount {
    #[allow(unused)]
    id: String,
    friends: Vec<FriendInfo>,
    // Reserved
    _cool_down: DateTime<Utc>,
}

const RESERVED_NOTFOUND_FRIEND_CODE: &str = "123456789";

static LOGIN_ACCOUNTS: OnceLock<RwLock<Option<HashMap<String, MockAccount>>>> = OnceLock::new();
static FAIL_CHANCE: OnceLock<f64> = OnceLock::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("initializing mocker");
    FAIL_CHANCE.get_or_init(|| {
        env::var("FAIL_CHANCE")
            .map_err(|e| anyhow!(e))
            .and_then(|it| {
                it.parse()
                    .map_err(|e: std::num::ParseFloatError| anyhow!(e))
            })
            .unwrap_or(0.3)
    });
    LOGIN_ACCOUNTS.get_or_init(|| RwLock::new(Some(HashMap::new())));
    let router = Router::new()
        .route("/", get(hello))
        .route("/healthz", get(healthz))
        .route("/auth/login", post(login))
        .route("/friend/me", get(list_friend))
        .route("/friend/me/add", post(add_friend))
        .route("/friend/me/delete", post(remove_friend))
        .route("/score/song/friend", get(get_rank_list));
    let addr = TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind 0.0.0.0:8080");
    tracing::info!("listening in 0.0.0.0:8080...");
    axum::serve(addr, router).await.unwrap();
}

async fn hello() -> impl IntoResponse {
    (StatusCode::OK, "Hello, this is mock awa")
}

fn fail_check() -> bool {
    let fail_chance = FAIL_CHANCE
        .get()
        .expect("FAIL_CHANCE should be initialized");
    rand::random_ratio((fail_chance * 1000.0) as u32, 1000)
}

fn generate_user_id(username: &str) -> String {
    let arr = sha2::Sha256::digest(username.as_bytes());
    let user_id = (arr[0] as u64 + arr[1] as u64).cast_signed();
    user_id.to_string()
}

async fn login(headers: HeaderMap) -> impl IntoResponse {
    let auth_header = headers.get(header::AUTHORIZATION).unwrap();
    let auth_header = &auth_header.to_str().unwrap()["Basic ".len()..];
    let auth_header = base64::prelude::BASE64_STANDARD
        .decode(auth_header)
        .unwrap();
    let auth_header_value = str::from_utf8(&auth_header).unwrap();
    let auth_header_value: Vec<_> = auth_header_value.split(':').collect();
    let (username, _) = (auth_header_value[0], auth_header_value[1]);
    // 为方便起见，mock不校验密码
    if fail_check() {
        tracing::info!("login[fail]: user_name: {username}");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(LoginResult::Failed {
                success: false,
                error_code: 0,
            }),
        );
    }
    let user_id = generate_user_id(username);
    LOGIN_ACCOUNTS
        .try_write(|it| {
            let mut map = it.expect("should initialized");
            map.insert(
                user_id.clone(),
                MockAccount {
                    id: user_id.clone(),
                    friends: Vec::new(),
                    _cool_down: Utc::now()
                        + chrono::Duration::from_std(Duration::from_secs(3))
                            .expect("should not happen"),
                },
            );
            Some(map)
        })
        .expect("should success");
    tracing::info!("login: user_name: {username}, id: {user_id}, token: {user_id}");
    (
        StatusCode::OK,
        Json(LoginResult::Success {
            success: true,
            user_id: user_id.parse().expect("should parse success"),
            access_token: user_id,
            token_type: "Bearer".to_owned(),
        }),
    )
}

async fn add_friend(headers: HeaderMap, form: Form<FriendAddForm>) -> impl IntoResponse {
    // 跳过鉴权步骤
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 114514,
            }),
        );
    };
    let i_header = i_header.to_str().expect("should success");
    if fail_check() {
        tracing::info!(
            "add_friend[fail_429_before_add]: i: {i_header}, target: {}",
            form.friend_code
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 0,
            }),
        );
    }
    create_empty_default_account(i_header);
    if form.friend_code == RESERVED_NOTFOUND_FRIEND_CODE {
        return (
            StatusCode::NOT_FOUND,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 401,
            }),
        );
    }
    let generated_user_id = generate_user_id(&form.friend_code);
    if LOGIN_ACCOUNTS
        .try_read(|map| {
            let account = map.get(i_header).expect("should success");
            account
                .friends
                .iter()
                .any(|friend| friend.user_id.to_string() == generated_user_id)
        })
        .expect("should success")
    {
        tracing::info!(
            "add_friend[duplicated]: i: {i_header}, target: {}",
            form.friend_code
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 12345,
            }),
        );
    }
    let mut maxed_out = false;
    let mut duplicated = false;
    LOGIN_ACCOUNTS
        .try_write(|it| {
            let mut map = it.expect("should be initialized");
            let account = map.get_mut(i_header).expect("should success");
            let friends = &mut account.friends;
            if friends.len() >= 2 {
                maxed_out = true;
                return Some(map);
            }
            let target_user_id = generated_user_id.parse().expect("should success");
            if friends.iter().any(|it| it.user_id == target_user_id) {
                duplicated = true;
                return Some(map);
            }
            friends.push(FriendInfo {
                name: "Mock friend".to_owned(),
                user_id: target_user_id,
                rating: 0,
                character: 0,
                is_char_uncapped: false,
                is_char_uncapped_override: false,
            });
            Some(map)
        })
        .expect("should success");
    if duplicated {
        tracing::info!(
            "add_friend[duplicated]: i: {i_header}, target: {}",
            form.friend_code
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 40102,
            }),
        );
    }
    if maxed_out {
        tracing::info!(
            "add_friend[maxed_out]: i: {i_header}, target: {}",
            form.friend_code
        );
        return (
            StatusCode::NOT_FOUND,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 40101,
            }),
        );
    }
    LOGIN_ACCOUNTS
        .try_read(|it| {
            let account = it.get(i_header).expect("should success");
            tracing::info!(
                "add_friend: i: {i_header}, target: {}, real_len: {}",
                form.friend_code,
                account.friends.len()
            );
            (
                StatusCode::OK,
                Json(FriendModifyResult::Success {
                    success: true,
                    value: FriendsResult {
                        friends: account.friends.clone(),
                    },
                }),
            )
        })
        .expect("should success")
}

async fn remove_friend(headers: HeaderMap, form: Form<FriendRemoveForm>) -> impl IntoResponse {
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 0,
            }),
        );
    };
    let i_header = i_header.to_str().expect("should success");
    if fail_check() {
        tracing::info!(
            "remove_friend[fail_429_before_remove]: i: {i_header}, target: {}",
            form.friend_id
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 0,
            }),
        );
    }
    create_empty_default_account(i_header);
    let mut contains_key = false;
    // if fail_check() {
    //     tracing::info!(
    //         "remove_friend[fail_empty_before_remove]: i: {i_header}, target: {}",
    //         form.friend_id
    //     );
    //     return (
    //         StatusCode::OK,
    //         Json(FriendModifyResult::Success {
    //             success: true,
    //             value: FriendsResult {
    //                 friends: Vec::new(),
    //             },
    //         }),
    //     );
    // }
    LOGIN_ACCOUNTS
        .try_write(|it| {
            let mut account_map = it.expect("should be initialized");
            let Some(account) = account_map.get_mut(i_header) else {
                return Some(account_map);
            };
            let friend = account
                .friends
                .iter()
                .enumerate()
                .find(|(_, it)| it.user_id.to_string() == form.friend_id);
            contains_key = friend.is_some();
            if let Some((index, _)) = friend {
                account.friends.remove(index);
            }
            Some(account_map)
        })
        .expect("should success");
    if fail_check() {
        tracing::info!(
            "remove_friend[fail_empty_after_remove]: i: {i_header}, target: {}",
            form.friend_id
        );
        return (
            StatusCode::OK,
            Json(FriendModifyResult::Success {
                success: true,
                value: FriendsResult {
                    friends: Vec::new(),
                },
            }),
        );
    }
    LOGIN_ACCOUNTS
        .try_read(|it| {
            if let Some(account) = it.get(i_header) {
                if contains_key {
                    tracing::info!("remove_friend: i: {i_header}, target: {}", form.friend_id);
                    (
                        StatusCode::OK,
                        Json(FriendModifyResult::Success {
                            success: true,
                            value: FriendsResult {
                                friends: account.friends.clone(),
                            },
                        }),
                    )
                } else {
                    tracing::info!(
                        "remove_friend[not_added]: i: {i_header}, target: {}",
                        form.friend_id
                    );
                    (
                        StatusCode::BAD_REQUEST,
                        Json(FriendModifyResult::Failed {
                            success: false,
                            error_code: 0,
                        }),
                    )
                }
            } else {
                tracing::info!(
                    "remove_friend[bad_account]: i: {i_header}, target: {}",
                    form.friend_id
                );
                (
                    StatusCode::BAD_REQUEST,
                    Json(FriendModifyResult::Failed {
                        success: false,
                        error_code: 0,
                    }),
                )
            }
        })
        .expect("should success")
}

async fn list_friend(headers: HeaderMap) -> impl IntoResponse {
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: -11,
            }),
        );
    };
    let i_header = i_header.to_str().expect("should success");
    if fail_check() {
        tracing::info!("list_friend[fail_429_before_list]: i: {i_header}",);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(FriendModifyResult::Failed {
                success: false,
                error_code: 0,
            }),
        );
    }
    create_empty_default_account(i_header);
    if fail_check() {
        tracing::info!("list_friend[fail_empty_before_list]: i: {i_header}",);
        (
            StatusCode::OK,
            Json(FriendModifyResult::Success {
                success: true,
                value: FriendsResult { friends: vec![] },
            }),
        )
    } else {
        LOGIN_ACCOUNTS
            .try_read(|it| {
                let Some(account) = it.get(i_header) else {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(FriendModifyResult::Failed {
                            success: false,
                            error_code: 0,
                        }),
                    );
                };
                tracing::info!(
                    "list_friend: i: {i_header} true_length: {}",
                    account.friends.len()
                );
                (
                    StatusCode::OK,
                    Json(FriendModifyResult::Success {
                        success: true,
                        value: FriendsResult {
                            friends: account.friends.clone(),
                        },
                    }),
                )
            })
            .expect("should success")
    }
}

async fn get_rank_list(
    Query(query): Query<RankListQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 依旧跳过鉴权
    if fail_check() {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RankListFetchResult::Failed {
                success: false,
                error_code: 0,
            }),
        );
    }
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(RankListFetchResult::Failed {
                success: false,
                error_code: -11,
            }),
        );
    };
    let i_header = i_header.to_str().expect("should success");
    create_empty_default_account(i_header);
    LOGIN_ACCOUNTS
        .try_read(|it| {
            // let mut output_vec = Vec::<RankListResult>::new();
            let Some(account) = it.get(i_header) else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(RankListFetchResult::Failed {
                        success: false,
                        error_code: -1,
                    }),
                );
            };
            let output_vec = account
                .friends
                .iter()
                .map(|it| RankListResult {
                    song_id: query.song_id.clone(),
                    difficulty: query.difficulty.parse().expect("should success"),
                    user_id: it.user_id,
                    score: 114514,
                    score_below_max: 1919810,
                    shiny_perfect_count: 10,
                    perfect_count: 11,
                    near_count: 12,
                    miss_count: 13,
                    clear_type: 14,
                    best_clear_type: 15,
                    health: 16,
                    time_played: 177,
                    modifier: 13,
                    name: it.name.clone(),
                    character: it.character,
                    is_skill_sealed: it.is_char_uncapped,
                    is_char_uncapped: it.is_char_uncapped_override,
                    icon: "fuck".to_owned(),
                    rank: it.rating,
                })
                .collect::<Vec<_>>();
            (
                StatusCode::OK,
                Json(RankListFetchResult::Success {
                    success: true,
                    value: output_vec,
                }),
            )
        })
        .expect("should success")
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"health":"ok"})))
}

fn create_empty_default_account(user_id: &str) {
    LOGIN_ACCOUNTS
        .try_write(|it| {
            let mut map = it.expect("should be Some");
            if !map.contains_key(user_id) {
                map.insert(
                    user_id.to_string(),
                    MockAccount {
                        id: user_id.to_string(),
                        friends: vec![],
                        _cool_down: Utc::now(),
                    },
                );
            }
            Some(map)
        })
        .expect("should success");
}
