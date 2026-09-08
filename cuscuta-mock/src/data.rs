use cuscuta_common::api::xxxxxx::FriendInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(untagged)]
pub enum FriendModifyResult {
    Success { success: bool, value: FriendsResult },
    Failed { success: bool, error_code: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendsResult {
    pub friends: Vec<FriendInfo>,
}

#[derive(Deserialize)]
pub struct FriendAddForm {
    pub friend_code: String,
}

#[derive(Deserialize)]
pub struct FriendRemoveForm {
    pub friend_id: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum LoginResult {
    Success {
        success: bool,
        user_id: i64,
        access_token: String,
        token_type: String,
    },
    Failed {
        success: bool,
        error_code: i32,
    },
}

#[derive(Deserialize)]
pub struct RankListQuery {
    pub song_id: String,
    pub difficulty: String,
    #[allow(unused)]
    pub start: String,
    #[allow(unused)]
    pub limit: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum RankListFetchResult {
    Success {
        success: bool,
        value: Vec<RankListResult>,
    },
    Failed {
        success: bool,
        error_code: i32,
    },
}

#[derive(Serialize, Default)]
pub struct RankListResult {
    pub song_id: String,
    pub difficulty: i64,
    pub user_id: i64,
    pub score: i64,
    pub score_below_max: i64,
    pub shiny_perfect_count: i64,
    pub perfect_count: i64,
    pub near_count: i64,
    pub miss_count: i64,
    pub clear_type: i64,
    pub best_clear_type: i64,
    pub health: i64,
    pub time_played: i64,
    pub modifier: i64,
    pub name: String,
    pub character: i64,
    pub is_skill_sealed: bool,
    pub is_char_uncapped: bool,
    pub icon: String,
    pub rank: i64,
}
