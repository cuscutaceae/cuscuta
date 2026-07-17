use serde::Deserialize;

/// xxxxxx的数据版本信息
#[derive(Debug, Deserialize, Clone)]
pub struct BundleData {
    /// bundle的版本
    #[serde(rename = "versionNumber")]
    pub version_number: String,

    /// 应用的版本
    #[serde(rename = "applicationVersionNumber")]
    pub application_version_number: String,
}

/// 曲目的难度信息
#[derive(Debug, Deserialize, Clone)]
pub struct Difficulty {
    /// 难度等级
    #[serde(rename = "ratingClass")]
    pub rating_class: i32,

    /// 难度定数（粗略）
    #[serde(rename = "rating")]
    pub rating: i32,
}

/// 曲目的适配数据模型，其`difficulties`可能为`None`
#[derive(Debug, Deserialize)]
pub struct SongRaw {
    /// 曲目的数字id
    #[serde(rename = "idx")]
    pub idx: i32,

    /// 曲目的字符串id
    #[serde(rename = "id")]
    pub id: String,

    /// 曲目的难度信息
    #[serde(rename = "difficulties")]
    pub difficulties: Option<Vec<Difficulty>>,
}

/// 曲目信息
#[derive(Clone)]
pub struct Song {
    /// 曲目的数字id
    pub idx: i32,

    /// 曲目的字符串id
    pub id: String,

    /// 曲目的难度信息
    pub difficulties: Vec<Difficulty>,
}

/// 曲目信息的适配数据模型（顶层）
#[derive(Debug, Deserialize)]
pub struct SongsResult {
    /// 曲目信息
    #[serde(rename = "songs")]
    pub songs: Vec<SongRaw>,
}

impl From<SongRaw> for Option<Song> {
    fn from(value: SongRaw) -> Self {
        value.difficulties.map(|it| Song {
            idx: value.idx,
            id: value.id,
            difficulties: it,
        })
    }
}
