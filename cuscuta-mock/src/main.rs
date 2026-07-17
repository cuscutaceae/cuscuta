//! cuscuta的mock应用
//!
//! 出于某些历史原因，它并不依赖`cuscuta-common`

mod data;

use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

use axum::{
    Form, Json, Router,
    extract::Query,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use base64::Engine;
use chrono::Utc;
use cuscuta_common::api::xxxxxx::FriendInfo;
use rand::{RngExt, seq::SliceRandom};
use serde_json::json;
use tokio::net::TcpListener;

use crate::data::{
    FriendAddForm, FriendModifyResult, FriendModifyResultFailed, FriendModifyResultSuccess,
    FriendRemoveForm, FriendsResult, LoginResult, LoginResultFailed, LoginResultSuccess,
    RankListFetchResult, RankListFetchResultFailed, RankListFetchResultSuccess, RankListQuery,
    RankListResult, check_env_bool,
};

static FRIEND_INFO: OnceLock<RwLock<HashMap<String, Vec<FriendInfo>>>> = OnceLock::new();
static VALID_ACCOUNT: &[(&str, &str, &str)] = &[
    // username, password, user_id
    ("123456", "123456", "123456"),
    ("234567", "234567", "234567"),
    ("345678", "345678", "345678"),
    ("456789", "456789", "456789"),
    ("nofyso", "Chensing", "114514"),
];

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("initializing mocker");
    initialize_friends();
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
    log::info!("listening in 0.0.0.0:8080...");
    axum::serve(addr, router).await.unwrap();
}

async fn hello() -> impl IntoResponse {
    (StatusCode::OK, "Hello")
}

fn initialize_friends() {
    let mut x = FRIEND_INFO
        .get_or_init(|| RwLock::new(HashMap::new()))
        .try_write()
        .unwrap();
    for it in VALID_ACCOUNT {
        x.insert(
            it.2.to_string(),
            vec![FriendInfo {
                name: "nofyso".into(),
                user_id: 9999999,
                rating: 10,
                character: 0,
                is_char_uncapped: true,
                is_char_uncapped_override: true,
            }],
        );
    }
}

fn clone_internal_friends() -> HashMap<String, Vec<FriendInfo>> {
    FRIEND_INFO
        .get_or_init(|| RwLock::new(HashMap::new()))
        .try_read()
        .unwrap()
        .clone()
}

fn internal_read_friends(id: String) -> FriendModifyResult {
    let x = FRIEND_INFO
        .get_or_init(|| RwLock::new(HashMap::new()))
        .try_read()
        .unwrap();
    FriendModifyResult::Success(FriendModifyResultSuccess {
        success: true,
        value: FriendsResult {
            friends: x.get(&id).unwrap().clone(),
        },
    })
}

fn internal_write_friends<F>(id: String, mut f: F) -> FriendModifyResult
where
    F: FnMut(&mut HashMap<String, Vec<FriendInfo>>) -> Option<FriendModifyResult>,
{
    let mut x = FRIEND_INFO
        .get_or_init(|| RwLock::new(HashMap::new()))
        .try_write()
        .unwrap();
    let failed = f(&mut x);
    if let Some(failed) = failed {
        return failed;
    }
    FriendModifyResult::Success(FriendModifyResultSuccess {
        success: true,
        value: FriendsResult {
            friends: x.get(&id).unwrap().clone(),
        },
    })
}

async fn login(headers: HeaderMap) -> impl IntoResponse {
    let always_fail = check_env_bool("CUSCUTA_LOGIN_ALWAYS_FAIL");
    let auth_header = headers.get(header::AUTHORIZATION).unwrap();
    let auth_header = &auth_header.to_str().unwrap()["Basic ".len()..];
    let auth_header = base64::prelude::BASE64_STANDARD
        .decode(auth_header)
        .unwrap();
    let auth_header_value = str::from_utf8(&auth_header).unwrap();
    let auth_header_value: Vec<_> = auth_header_value.split(':').collect();
    let value = (auth_header_value[0], auth_header_value[1]);
    let account = if always_fail {
        None
    } else {
        VALID_ACCOUNT
            .iter()
            .find(|it| it.0 == value.0 && it.1 == value.1)
    };

    if let Some(account) = account {
        let token = get_current_token();
        log::info!("login: success: id:{}, token:{token}", account.2);
        (
            StatusCode::OK,
            Json(LoginResult::Success(LoginResultSuccess {
                success: true,
                user_id: account.2.parse().unwrap(),
                access_token: token,
                token_type: "Bearer".into(),
            })),
        )
    } else {
        log::info!("login: failed, always:{always_fail}");
        (
            StatusCode::NOT_FOUND,
            Json(LoginResult::Failed(LoginResultFailed {
                success: false,
                error_code: 104,
            })),
        )
    }
}

async fn add_friend(headers: HeaderMap, form: Form<FriendAddForm>) -> impl IntoResponse {
    if !check_token(&headers) {
        log::warn!("add_friend: check_token failed");
        return (
            StatusCode::UNAUTHORIZED,
            Json(FriendModifyResult::Failed(FriendModifyResultFailed {
                success: false,
                error_code: 1145,
            })),
        );
    }
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed(FriendModifyResultFailed {
                success: false,
                error_code: -11,
            })),
        );
    };
    if form.friend_code == "123456789" {
        return (
            StatusCode::NOT_FOUND,
            Json(FriendModifyResult::Failed(FriendModifyResultFailed {
                success: false,
                error_code: 404,
            })),
        );
    }
    let i_header = i_header.to_str().unwrap().to_string();
    //TODO add random not found fail
    let result = internal_write_friends(
        i_header.clone(),
        |it: &mut HashMap<String, Vec<FriendInfo>>| {
            let it = it.get_mut(&i_header).unwrap();
            if it
                .iter()
                .any(|info| info.user_id.to_string() == form.friend_code)
            {
                log::warn!("add_friend: failed, duplicated addition");
                return FriendModifyResult::Failed(FriendModifyResultFailed {
                    success: false,
                    error_code: 1,
                })
                .into();
            }
            it.push(FriendInfo {
                name: format!("a_friend_{}", form.friend_code),
                user_id: form.friend_code.parse::<i64>().unwrap(),
                rating: 1234,
                character: 0,
                is_char_uncapped: true,
                is_char_uncapped_override: true,
            });
            log::info!("add_friend: success, friend_code:{}", form.friend_code);
            None
        },
    );
    let code = match &result {
        FriendModifyResult::Success(_) => StatusCode::OK,
        FriendModifyResult::Failed(_) => StatusCode::BAD_REQUEST,
    };
    (code, Json(result))
}

async fn remove_friend(headers: HeaderMap, form: Form<FriendRemoveForm>) -> impl IntoResponse {
    if !check_token(&headers) {
        log::warn!("remove_friend: check_token failed");
        return (
            StatusCode::UNAUTHORIZED,
            Json(FriendModifyResult::Failed(FriendModifyResultFailed {
                success: false,
                error_code: 1145,
            })),
        );
    }
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed(FriendModifyResultFailed {
                success: false,
                error_code: -11,
            })),
        );
    };
    let i_header = i_header.to_str().unwrap().to_string();
    //TODO add random not found fail
    let result = internal_write_friends(
        i_header.clone(),
        |it: &mut HashMap<String, Vec<FriendInfo>>| {
            let it = it.get_mut(&i_header).unwrap();
            let index = it
                .iter()
                .enumerate()
                .find(|it| it.1.user_id.to_string() == form.friend_id)
                .map(|it| it.0);
            match index {
                Some(index) => {
                    log::info!("remove_friend: success, {}", form.friend_id);
                    it.remove(index);
                    None
                }
                None => {
                    log::warn!(
                        "remove_friend: failed, friend not found: {} in {:?}",
                        form.friend_id,
                        it
                    );
                    Some(FriendModifyResult::Failed(FriendModifyResultFailed {
                        success: false,
                        error_code: 401,
                    }))
                }
            }
        },
    );
    let code = match &result {
        FriendModifyResult::Success(_) => StatusCode::OK,
        FriendModifyResult::Failed(_) => StatusCode::NOT_FOUND,
    };
    (code, Json(result))
}

async fn list_friend(headers: HeaderMap) -> impl IntoResponse {
    if !check_token(&headers) {
        log::warn!("list_friend: check_token failed");
        return (
            StatusCode::UNAUTHORIZED,
            Json(FriendModifyResult::Failed(FriendModifyResultFailed {
                success: false,
                error_code: 1145,
            })),
        );
    }
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(FriendModifyResult::Failed(FriendModifyResultFailed {
                success: false,
                error_code: -11,
            })),
        );
    };
    let i_header = i_header.to_str().unwrap().to_string();
    let result = internal_read_friends(i_header);
    let status_code = match &result {
        FriendModifyResult::Success(result) => {
            log::info!("list_friend: success, {:?}", result.value);
            StatusCode::OK
        }
        FriendModifyResult::Failed(_) => {
            log::error!("list_friend: failed, this should never happen...");
            StatusCode::FORBIDDEN
        } //this never happen...
    };
    (status_code, Json(result))
}

async fn get_rank_list(
    Query(query): Query<RankListQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_token(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(RankListFetchResult::Failed(RankListFetchResultFailed {
                success: false,
                error_code: 1145,
            })),
        );
    }
    let mut rand = rand::rng();
    if rand.random_range(0..5) == 0 {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RankListFetchResult::Failed(RankListFetchResultFailed {
                success: false,
                error_code: 555,
            })),
        );
    }
    let Some(i_header) = headers.get("i") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(RankListFetchResult::Failed(RankListFetchResultFailed {
                success: false,
                error_code: -11,
            })),
        );
    };
    let i_header = i_header.to_str().unwrap().to_string();
    let has_result = true; //rand.random::<bool>();
    let mut output_vec = Vec::<RankListResult>::new();
    let friends = clone_internal_friends();
    let friends = friends.get(&i_header).unwrap();
    if has_result && !friends.is_empty() {
        // let friend = friends.get(rand.random_range(0..friends.len())).unwrap();
        for friend in friends {
            output_vec.push(RankListResult {
                song_id: query.song_id.clone(),
                difficulty: query.difficulty.parse().unwrap(),
                user_id: friend.user_id,
                score: rand.random(),
                score_below_max: rand.random(),
                shiny_perfect_count: rand.random(),
                perfect_count: rand.random(),
                near_count: rand.random(),
                miss_count: rand.random(),
                clear_type: rand.random(),
                best_clear_type: rand.random(),
                health: rand.random(),
                time_played: rand.random(),
                modifier: rand.random(),
                name: friend.name.clone(),
                character: friend.character,
                is_skill_sealed: rand.random(),
                is_char_uncapped: rand.random(),
                icon: "".into(),
                rank: rand.random(),
            });
        }
    }
    for _ in 0..rand
        .random_range(1..6usize)
        .max(query.limit.parse().unwrap())
    {
        output_vec.push(RankListResult {
            song_id: query.song_id.clone(),
            difficulty: query.difficulty.parse().unwrap(),
            user_id: 12345,
            score: rand.random(),
            score_below_max: rand.random(),
            shiny_perfect_count: rand.random(),
            perfect_count: rand.random(),
            near_count: rand.random(),
            miss_count: rand.random(),
            clear_type: rand.random(),
            best_clear_type: rand.random(),
            health: rand.random(),
            time_played: rand.random(),
            modifier: rand.random(),
            name: "Random_name".into(),
            character: 0,
            is_skill_sealed: rand.random(),
            is_char_uncapped: rand.random(),
            icon: "".into(),
            rank: rand.random(),
        });
    }
    output_vec.shuffle(&mut rand);
    (
        StatusCode::OK,
        Json(RankListFetchResult::Success(RankListFetchResultSuccess {
            success: true,
            value: output_vec,
        })),
    )
}

fn check_token(headers: &HeaderMap) -> bool {
    let token = headers.get("Authorization").and_then(|it| it.to_str().ok());
    let Some(token) = token else {
        return false;
    };
    if token.len() <= "Bearer ".len() {
        return false;
    }
    let token = &token["Bearer ".len()..];
    let Ok(time) = base64::prelude::BASE64_URL_SAFE.decode(token) else {
        return false;
    };
    let time = i64::from_le_bytes(time[..8].try_into().unwrap());
    let now = Utc::now();
    let now_time = now.timestamp_millis();
    now_time - time <= 1000 * 60 * 60 * 24 * 30 && time <= now_time
}

fn get_current_token() -> String {
    let now = Utc::now();
    let time = now.timestamp_millis();
    let buf = time.to_le_bytes();
    base64::prelude::BASE64_URL_SAFE.encode(buf)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"health":"ok"})))
}
