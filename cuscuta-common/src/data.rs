use serde::{Deserialize, Deserializer};

/// xxxxxx的数据版本信息
#[derive(Debug, Clone)]
pub struct AppVersionData {
    /// App的版本
    pub version: String,
}

impl<'de> Deserialize<'de> for AppVersionData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 先反序列化成中间结构，再取出 a
        #[derive(Deserialize)]
        struct Wrapper {
            value: InnerRaw,
        }
        #[derive(Deserialize)]
        struct InnerRaw {
            version: String,
        }

        let w = Wrapper::deserialize(deserializer)?;
        Ok(Self {
            version: w.value.version,
        })
    }
}

/// scirpophaga 的输出JSON数据格式
#[derive(Debug, Deserialize, Clone)]
pub struct ScirpophagaData {
    /// c2 常量
    pub c2: String,

    /// 一般API路径前缀
    pub common_path: String,

    /// 认证API路径前缀
    pub auth_path: String,
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
