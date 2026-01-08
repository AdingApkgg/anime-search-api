//! Bangumi API 集成
//! https://bangumi.github.io/api/
//! User Agent 规范: https://github.com/bangumi/api/blob/master/docs-raw/user%20agent.md

use crate::http_client::HTTP_CLIENT;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

const BANGUMI_API: &str = "https://api.bgm.tv";
const USER_AGENT: &str = "kirito/anime-search (https://github.com/Moe-Sakura/anime-search-api)";

// Bangumi 应用凭证 (https://bgm.tv/dev/app)
#[allow(dead_code)]
const APP_ID: &str = "bgm5356695eacc14314f";
#[allow(dead_code)]
const APP_SECRET: &str = "af886557f6083a06d0ba9614f28afee5";

/// 获取有效的 access token
/// 优先使用用户提供的 token，否则使用服务端配置的默认 token
pub fn get_effective_token(user_token: Option<&str>) -> Option<&str> {
    // 优先使用用户提供的 token
    if let Some(token) = user_token {
        if !token.is_empty() {
            return Some(token);
        }
    }
    
    // 尝试从环境变量获取服务端默认 token
    get_server_token()
}

/// 获取服务端配置的默认 token (从环境变量 BANGUMI_ACCESS_TOKEN)
fn get_server_token() -> Option<&'static str> {
    use once_cell::sync::Lazy;
    static SERVER_TOKEN: Lazy<Option<String>> = Lazy::new(|| {
        std::env::var("BANGUMI_ACCESS_TOKEN").ok().filter(|s| !s.is_empty())
    });
    SERVER_TOKEN.as_deref()
}

// ============================================================================
// 公共类型定义
// ============================================================================

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiSearchResult {
    pub results: i32,
    #[serde(default)]
    pub list: Vec<BangumiSubject>,
}

/// 条目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiSubject {
    pub id: i64,
    pub url: String,
    #[serde(rename = "type")]
    pub subject_type: i32,
    pub name: String,
    #[serde(default)]
    pub name_cn: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub air_date: String,
    #[serde(default)]
    pub air_weekday: i32,
    #[serde(default)]
    pub images: Option<BangumiImages>,
    #[serde(default)]
    pub eps: Option<i32>,
    #[serde(default)]
    pub eps_count: Option<i32>,
    #[serde(default)]
    pub rating: Option<BangumiRating>,
    #[serde(default)]
    pub rank: Option<i32>,
    #[serde(default)]
    pub collection: Option<BangumiCollection>,
    #[serde(default)]
    pub tags: Option<Vec<BangumiTag>>,
    #[serde(default)]
    pub infobox: Option<Vec<InfoboxItem>>,
    #[serde(default)]
    pub total_episodes: Option<i32>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub nsfw: Option<bool>,
}

/// 图片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiImages {
    pub large: String,
    pub common: String,
    pub medium: String,
    pub small: String,
    pub grid: String,
}

/// 评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiRating {
    #[serde(default)]
    pub rank: Option<i32>,  // rank 可能在这里或在顶层 Subject.rank
    #[serde(default)]
    pub total: i32,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub count: Option<BangumiRatingCount>,
}

/// 评分分布
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiRatingCount {
    #[serde(rename = "1", default)]
    pub s1: i32,
    #[serde(rename = "2", default)]
    pub s2: i32,
    #[serde(rename = "3", default)]
    pub s3: i32,
    #[serde(rename = "4", default)]
    pub s4: i32,
    #[serde(rename = "5", default)]
    pub s5: i32,
    #[serde(rename = "6", default)]
    pub s6: i32,
    #[serde(rename = "7", default)]
    pub s7: i32,
    #[serde(rename = "8", default)]
    pub s8: i32,
    #[serde(rename = "9", default)]
    pub s9: i32,
    #[serde(rename = "10", default)]
    pub s10: i32,
}

/// 收藏统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiCollection {
    #[serde(default)]
    pub wish: i32,
    #[serde(default)]
    pub collect: i32,
    #[serde(default)]
    pub doing: i32,
    #[serde(default)]
    pub on_hold: i32,
    #[serde(default)]
    pub dropped: i32,
}

/// 标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BangumiTag {
    pub name: String,
    pub count: i32,
}

/// Infobox 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoboxItem {
    pub key: String,
    pub value: Value,
}

/// 每日放送
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarItem {
    pub weekday: Weekday,
    pub items: Vec<BangumiSubject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weekday {
    pub en: String,
    pub cn: String,
    pub ja: String,
    pub id: i32,
}

// ============================================================================
// 用户相关类型
// ============================================================================

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    #[serde(default)]
    pub avatar: Option<UserAvatar>,
    #[serde(default)]
    pub sign: String,
    #[serde(default)]
    pub user_group: Option<i32>,
}

/// 用户头像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAvatar {
    pub large: String,
    pub medium: String,
    pub small: String,
}

// ============================================================================
// 收藏相关类型
// ============================================================================

/// 用户收藏
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCollection {
    pub subject_id: i64,
    pub subject: Option<BangumiSubject>,
    #[serde(rename = "type")]
    pub collection_type: i32,
    #[serde(default)]
    pub rate: i32,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub ep_status: i32,
    #[serde(default)]
    pub vol_status: i32,
    #[serde(default)]
    pub updated_at: String,
}

/// 用户收藏列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCollectionList {
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
    pub data: Vec<UserCollection>,
}

/// 收藏类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(i32)]
#[allow(dead_code)]
pub enum CollectionType {
    Wish = 1,     // 想看
    Collect = 2,  // 看过
    Doing = 3,    // 在看
    OnHold = 4,   // 搁置
    Dropped = 5,  // 抛弃
}

impl From<i32> for CollectionType {
    fn from(v: i32) -> Self {
        match v {
            1 => CollectionType::Wish,
            2 => CollectionType::Collect,
            3 => CollectionType::Doing,
            4 => CollectionType::OnHold,
            5 => CollectionType::Dropped,
            _ => CollectionType::Wish,
        }
    }
}

/// 修改收藏请求
#[derive(Debug, Clone, Serialize)]
pub struct CollectionModify {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub collection_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ep_status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

// ============================================================================
// 角色相关类型
// ============================================================================

/// 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub character_type: i32,
    #[serde(default)]
    pub images: Option<BangumiImages>,
    #[serde(default)]
    pub relation: Option<String>,
    #[serde(default)]
    pub actors: Option<Vec<Person>>,
}

/// 角色详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDetail {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub character_type: i32,
    #[serde(default)]
    pub images: Option<BangumiImages>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub infobox: Option<Vec<InfoboxItem>>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub blood_type: Option<i32>,
    #[serde(default)]
    pub birth_year: Option<i32>,
    #[serde(default)]
    pub birth_mon: Option<i32>,
    #[serde(default)]
    pub birth_day: Option<i32>,
    #[serde(default)]
    pub stat: Option<CharacterStat>,
}

/// 角色统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterStat {
    #[serde(default)]
    pub comments: i32,
    #[serde(default)]
    pub collects: i32,
}

// ============================================================================
// 人物相关类型
// ============================================================================

/// 人物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub person_type: i32,
    #[serde(default)]
    pub images: Option<PersonImages>,
    #[serde(default)]
    pub relation: Option<String>,
    #[serde(default)]
    pub career: Option<Vec<String>>,
}

/// 人物图片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonImages {
    pub large: String,
    pub medium: String,
    pub small: String,
    pub grid: String,
}

/// 人物详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDetail {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub person_type: i32,
    #[serde(default)]
    pub career: Vec<String>,
    #[serde(default)]
    pub images: Option<PersonImages>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub last_modified: String,
    #[serde(default)]
    pub infobox: Option<Vec<InfoboxItem>>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub blood_type: Option<i32>,
    #[serde(default)]
    pub birth_year: Option<i32>,
    #[serde(default)]
    pub birth_mon: Option<i32>,
    #[serde(default)]
    pub birth_day: Option<i32>,
    #[serde(default)]
    pub stat: Option<PersonStat>,
}

/// 人物统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonStat {
    #[serde(default)]
    pub comments: i32,
    #[serde(default)]
    pub collects: i32,
}

// ============================================================================
// 章节相关类型
// ============================================================================

/// 章节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: i64,
    #[serde(rename = "type")]
    pub episode_type: i32,
    pub name: String,
    #[serde(default)]
    pub name_cn: String,
    #[serde(default)]
    pub sort: f64,
    #[serde(default)]
    pub ep: Option<f64>,
    #[serde(default)]
    pub airdate: String,
    #[serde(default)]
    pub comment: i32,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub disc: i32,
    #[serde(default)]
    pub duration_seconds: Option<i32>,
    #[serde(default)]
    pub subject_id: Option<i64>,
}

/// 章节列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeList {
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
    pub data: Vec<Episode>,
}

/// 用户章节收藏
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UserEpisodeCollection {
    pub episode: Episode,
    #[serde(rename = "type")]
    pub collection_type: i32,
}

/// 章节收藏类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(i32)]
#[allow(dead_code)]
pub enum EpisodeCollectionType {
    None = 0,     // 未收藏
    Wish = 1,     // 想看
    Done = 2,     // 看过
    Dropped = 3,  // 抛弃
}

// ============================================================================
// 关联条目
// ============================================================================

/// 关联条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedSubject {
    pub id: i64,
    #[serde(rename = "type")]
    pub subject_type: i32,
    pub name: String,
    #[serde(default)]
    pub name_cn: String,
    #[serde(default)]
    pub images: Option<BangumiImages>,
    pub relation: String,
}

// ============================================================================
// 目录相关类型
// ============================================================================

/// 目录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub desc: String,
    pub total: i32,
    #[serde(default)]
    pub stat: Option<IndexStat>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub creator: Option<User>,
    #[serde(default)]
    pub ban: bool,
    #[serde(default)]
    pub nsfw: bool,
}

/// 目录统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStat {
    #[serde(default)]
    pub comments: i32,
    #[serde(default)]
    pub collects: i32,
}

/// 目录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSubject {
    #[serde(default)]
    pub added_at: String,
    #[serde(default)]
    pub comment: String,
    pub subject: BangumiSubject,
}

/// 目录条目列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSubjectList {
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
    pub data: Vec<IndexSubject>,
}

// ============================================================================
// v0 搜索类型
// ============================================================================

/// v0 搜索请求
#[derive(Debug, Clone, Serialize)]
pub struct SearchRequest {
    pub keyword: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<SearchFilter>,
}

/// 搜索过滤器
#[derive(Debug, Clone, Serialize)]
pub struct SearchFilter {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub subject_type: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub air_date: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
}

/// v0 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultV0 {
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
    pub data: Vec<BangumiSubject>,
}

// ============================================================================
// 简化类型 (用于前端)
// ============================================================================

/// 简化的动漫信息 (用于前端显示)
#[derive(Debug, Clone, Serialize)]
pub struct AnimeInfo {
    pub id: i64,
    pub name: String,
    pub name_cn: String,
    pub summary: String,
    pub air_date: String,
    pub image: String,
    pub url: String,
    pub score: Option<f64>,
    pub rank: Option<i32>,
}

impl From<BangumiSubject> for AnimeInfo {
    fn from(s: BangumiSubject) -> Self {
        Self {
            id: s.id,
            name: s.name,
            name_cn: s.name_cn,
            summary: s.summary,
            air_date: s.air_date,
            image: s.images.map(|i| i.medium).unwrap_or_default(),
            url: s.url,
            score: s.rating.as_ref().and_then(|r| if r.score > 0.0 { Some(r.score) } else { None }),
            // 优先使用顶层 rank，回退到 rating.rank
            rank: s.rank.or_else(|| s.rating.as_ref().and_then(|r| r.rank)),
        }
    }
}

// ============================================================================
// HTTP 请求辅助函数
// ============================================================================

/// 发送带认证的 GET 请求
async fn get_with_auth<T: for<'de> Deserialize<'de>>(url: &str, token: &str) -> anyhow::Result<T> {
    let response = HTTP_CLIENT
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {} - {}", response.status(), response.text().await.unwrap_or_default());
    }

    let result: T = response.json().await?;
    Ok(result)
}

/// 发送带认证的 POST 请求
#[allow(dead_code)]
async fn post_with_auth<T: for<'de> Deserialize<'de>, B: Serialize>(
    url: &str,
    token: &str,
    body: &B,
) -> anyhow::Result<T> {
    let response = HTTP_CLIENT
        .post(url)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {} - {}", response.status(), response.text().await.unwrap_or_default());
    }

    let result: T = response.json().await?;
    Ok(result)
}

/// 发送带认证的 POST 请求 (无响应体)
async fn post_with_auth_empty<B: Serialize>(url: &str, token: &str, body: &B) -> anyhow::Result<()> {
    let response = HTTP_CLIENT
        .post(url)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {} - {}", response.status(), response.text().await.unwrap_or_default());
    }

    Ok(())
}

/// 发送带认证的 PATCH 请求
async fn patch_with_auth<B: Serialize>(url: &str, token: &str, body: &B) -> anyhow::Result<()> {
    let response = HTTP_CLIENT
        .patch(url)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {} - {}", response.status(), response.text().await.unwrap_or_default());
    }

    Ok(())
}

/// 发送带认证的 DELETE 请求
async fn delete_with_auth(url: &str, token: &str) -> anyhow::Result<()> {
    let response = HTTP_CLIENT
        .delete(url)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {} - {}", response.status(), response.text().await.unwrap_or_default());
    }

    Ok(())
}

// ============================================================================
// 公开 API (无需认证)
// ============================================================================

/// 搜索动漫 (type=2)
/// 使用 responseGroup=large 获取完整信息（评分、排名等）
pub async fn search_anime(keyword: &str) -> anyhow::Result<BangumiSearchResult> {
    let url = format!(
        "{}/search/subject/{}?type=2&responseGroup=large",
        BANGUMI_API,
        urlencoding::encode(keyword)
    );

    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let result: BangumiSearchResult = response.json().await?;
    Ok(result)
}

/// 获取条目详情
pub async fn get_subject(id: i64) -> anyhow::Result<BangumiSubject> {
    let url = format!("{}/subject/{}", BANGUMI_API, id);

    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let subject: BangumiSubject = response.json().await?;
    Ok(subject)
}

/// 获取每日放送
pub async fn get_calendar() -> anyhow::Result<Vec<CalendarItem>> {
    let url = format!("{}/calendar", BANGUMI_API);

    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let calendar: Vec<CalendarItem> = response.json().await?;
    Ok(calendar)
}

/// 搜索并返回简化信息
pub async fn search_anime_simple(keyword: &str) -> Vec<AnimeInfo> {
    match search_anime(keyword).await {
        Ok(result) => result.list.into_iter().map(AnimeInfo::from).collect(),
        Err(e) => {
            warn!("Bangumi 搜索失败: {}", e);
            vec![]
        }
    }
}

// ============================================================================
// v0 API (公开/可选认证)
// ============================================================================

/// v0 条目搜索 (POST /v0/search/subjects)
pub async fn search_subjects_v0(
    request: &SearchRequest,
    limit: Option<i32>,
    offset: Option<i32>,
    token: Option<&str>,
) -> anyhow::Result<SearchResultV0> {
    let mut url = format!("{}/v0/search/subjects", BANGUMI_API);
    let mut params = vec![];
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let mut req = HTTP_CLIENT
        .post(&url)
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/json")
        .json(request);

    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let result: SearchResultV0 = response.json().await?;
    Ok(result)
}

/// 获取条目详情 v0 (GET /v0/subjects/{id})
pub async fn get_subject_v0(id: i64, token: Option<&str>) -> anyhow::Result<BangumiSubject> {
    let url = format!("{}/v0/subjects/{}", BANGUMI_API, id);

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let subject: BangumiSubject = response.json().await?;
    Ok(subject)
}

/// 获取条目角色 (GET /v0/subjects/{id}/characters)
pub async fn get_subject_characters(id: i64, token: Option<&str>) -> anyhow::Result<Vec<Character>> {
    let url = format!("{}/v0/subjects/{}/characters", BANGUMI_API, id);

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let chars: Vec<Character> = response.json().await?;
    Ok(chars)
}

/// 获取条目制作人员 (GET /v0/subjects/{id}/persons)
pub async fn get_subject_persons(id: i64, token: Option<&str>) -> anyhow::Result<Vec<Person>> {
    let url = format!("{}/v0/subjects/{}/persons", BANGUMI_API, id);

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let persons: Vec<Person> = response.json().await?;
    Ok(persons)
}

/// 获取条目关联条目 (GET /v0/subjects/{id}/subjects)
pub async fn get_subject_relations(id: i64, token: Option<&str>) -> anyhow::Result<Vec<RelatedSubject>> {
    let url = format!("{}/v0/subjects/{}/subjects", BANGUMI_API, id);

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let relations: Vec<RelatedSubject> = response.json().await?;
    Ok(relations)
}

/// 获取章节列表 (GET /v0/episodes)
pub async fn get_episodes(
    subject_id: i64,
    episode_type: Option<i32>,
    limit: Option<i32>,
    offset: Option<i32>,
    token: Option<&str>,
) -> anyhow::Result<EpisodeList> {
    let mut params = vec![format!("subject_id={}", subject_id)];
    if let Some(t) = episode_type {
        params.push(format!("type={}", t));
    }
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }

    let url = format!("{}/v0/episodes?{}", BANGUMI_API, params.join("&"));

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let episodes: EpisodeList = response.json().await?;
    Ok(episodes)
}

/// 获取章节详情 (GET /v0/episodes/{id})
pub async fn get_episode(id: i64, token: Option<&str>) -> anyhow::Result<Episode> {
    let url = format!("{}/v0/episodes/{}", BANGUMI_API, id);

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let episode: Episode = response.json().await?;
    Ok(episode)
}

/// 获取角色详情 (GET /v0/characters/{id})
pub async fn get_character(id: i64) -> anyhow::Result<CharacterDetail> {
    let url = format!("{}/v0/characters/{}", BANGUMI_API, id);

    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let character: CharacterDetail = response.json().await?;
    Ok(character)
}

/// 获取人物详情 (GET /v0/persons/{id})
pub async fn get_person(id: i64) -> anyhow::Result<PersonDetail> {
    let url = format!("{}/v0/persons/{}", BANGUMI_API, id);

    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let person: PersonDetail = response.json().await?;
    Ok(person)
}

/// 获取用户信息 (GET /v0/users/{username})
pub async fn get_user(username: &str) -> anyhow::Result<User> {
    let url = format!("{}/v0/users/{}", BANGUMI_API, urlencoding::encode(username));

    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let user: User = response.json().await?;
    Ok(user)
}

// ============================================================================
// 需要认证的 API
// ============================================================================

/// 获取当前用户信息 (GET /v0/me)
pub async fn get_me(token: &str) -> anyhow::Result<User> {
    let url = format!("{}/v0/me", BANGUMI_API);
    get_with_auth(&url, token).await
}

/// 获取用户收藏列表 (GET /v0/users/{username}/collections)
pub async fn get_user_collections(
    username: &str,
    subject_type: Option<i32>,
    collection_type: Option<i32>,
    limit: Option<i32>,
    offset: Option<i32>,
    token: &str,
) -> anyhow::Result<UserCollectionList> {
    let mut params = vec![];
    if let Some(t) = subject_type {
        params.push(format!("subject_type={}", t));
    }
    if let Some(t) = collection_type {
        params.push(format!("type={}", t));
    }
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }

    let mut url = format!("{}/v0/users/{}/collections", BANGUMI_API, urlencoding::encode(username));
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    get_with_auth(&url, token).await
}

/// 获取用户单个条目收藏 (GET /v0/users/{username}/collections/{subject_id})
pub async fn get_user_collection(
    username: &str,
    subject_id: i64,
    token: &str,
) -> anyhow::Result<UserCollection> {
    let url = format!(
        "{}/v0/users/{}/collections/{}",
        BANGUMI_API,
        urlencoding::encode(username),
        subject_id
    );
    get_with_auth(&url, token).await
}

/// 新增/修改用户收藏 (POST /v0/users/-/collections/{subject_id})
pub async fn add_collection(
    subject_id: i64,
    collection_type: i32,
    rate: Option<i32>,
    comment: Option<String>,
    private: Option<bool>,
    tags: Option<Vec<String>>,
    token: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/v0/users/-/collections/{}", BANGUMI_API, subject_id);
    let body = CollectionModify {
        collection_type: Some(collection_type),
        rate,
        ep_status: None,
        vol_status: None,
        comment,
        private,
        tags,
    };
    post_with_auth_empty(&url, token, &body).await
}

/// 修改用户收藏 (PATCH /v0/users/-/collections/{subject_id})
pub async fn update_collection(
    subject_id: i64,
    modify: &CollectionModify,
    token: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/v0/users/-/collections/{}", BANGUMI_API, subject_id);
    patch_with_auth(&url, token, modify).await
}

/// 获取章节收藏信息 (GET /v0/users/-/collections/{subject_id}/episodes)
pub async fn get_episode_collections(
    subject_id: i64,
    episode_type: Option<i32>,
    limit: Option<i32>,
    offset: Option<i32>,
    token: &str,
) -> anyhow::Result<Value> {
    let mut params = vec![];
    if let Some(t) = episode_type {
        params.push(format!("episode_type={}", t));
    }
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }

    let mut url = format!("{}/v0/users/-/collections/{}/episodes", BANGUMI_API, subject_id);
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    get_with_auth(&url, token).await
}

/// 更新章节收藏 (PUT /v0/users/-/collections/-/episodes/{episode_id})
pub async fn update_episode_collection(
    episode_id: i64,
    collection_type: i32,
    token: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/v0/users/-/collections/-/episodes/{}", BANGUMI_API, episode_id);
    let body = serde_json::json!({ "type": collection_type });

    let response = HTTP_CLIENT
        .put(&url)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {} - {}", response.status(), response.text().await.unwrap_or_default());
    }

    Ok(())
}

/// 收藏角色 (POST /v0/characters/{character_id}/collect)
pub async fn collect_character(character_id: i64, token: &str) -> anyhow::Result<()> {
    let url = format!("{}/v0/characters/{}/collect", BANGUMI_API, character_id);
    let body: serde_json::Value = serde_json::json!({});
    post_with_auth_empty(&url, token, &body).await
}

/// 取消收藏角色 (DELETE /v0/characters/{character_id}/collect)
pub async fn uncollect_character(character_id: i64, token: &str) -> anyhow::Result<()> {
    let url = format!("{}/v0/characters/{}/collect", BANGUMI_API, character_id);
    delete_with_auth(&url, token).await
}

/// 收藏人物 (POST /v0/persons/{person_id}/collect)
pub async fn collect_person(person_id: i64, token: &str) -> anyhow::Result<()> {
    let url = format!("{}/v0/persons/{}/collect", BANGUMI_API, person_id);
    let body: serde_json::Value = serde_json::json!({});
    post_with_auth_empty(&url, token, &body).await
}

/// 取消收藏人物 (DELETE /v0/persons/{person_id}/collect)
pub async fn uncollect_person(person_id: i64, token: &str) -> anyhow::Result<()> {
    let url = format!("{}/v0/persons/{}/collect", BANGUMI_API, person_id);
    delete_with_auth(&url, token).await
}

/// 获取目录详情 (GET /v0/indices/{index_id})
pub async fn get_index(index_id: i64, token: Option<&str>) -> anyhow::Result<Index> {
    let url = format!("{}/v0/indices/{}", BANGUMI_API, index_id);

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let index: Index = response.json().await?;
    Ok(index)
}

/// 获取目录条目 (GET /v0/indices/{index_id}/subjects)
pub async fn get_index_subjects(
    index_id: i64,
    limit: Option<i32>,
    offset: Option<i32>,
    token: Option<&str>,
) -> anyhow::Result<IndexSubjectList> {
    let mut params = vec![];
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }

    let mut url = format!("{}/v0/indices/{}/subjects", BANGUMI_API, index_id);
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let mut req = HTTP_CLIENT.get(&url).header("User-Agent", USER_AGENT);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let response = req.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Bangumi API 返回错误: {}", response.status());
    }

    let subjects: IndexSubjectList = response.json().await?;
    Ok(subjects)
}

/// 收藏目录 (POST /v0/indices/{index_id}/collect)
pub async fn collect_index(index_id: i64, token: &str) -> anyhow::Result<()> {
    let url = format!("{}/v0/indices/{}/collect", BANGUMI_API, index_id);
    let body: serde_json::Value = serde_json::json!({});
    post_with_auth_empty(&url, token, &body).await
}

/// 取消收藏目录 (DELETE /v0/indices/{index_id}/collect)
pub async fn uncollect_index(index_id: i64, token: &str) -> anyhow::Result<()> {
    let url = format!("{}/v0/indices/{}/collect", BANGUMI_API, index_id);
    delete_with_auth(&url, token).await
}
