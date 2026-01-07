mod bangumi;
mod core;
mod engine;
mod http_client;
mod rules;
mod types;
mod updater;

use axum::{
    body::Body,
    extract::{Multipart, Path, Query},
    http::{header, HeaderMap, Method, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::core::search_stream_with_rules_options;
use crate::rules::get_builtin_rules;

#[tokio::main]
async fn main() {
    // 初始化日志
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // CORS 配置
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    // 检查启动时是否自动更新规则
    if std::env::var("AUTO_UPDATE").unwrap_or_default() == "1" {
        info!("📡 正在检查规则更新...");
        let result = updater::update_rules().await;
        info!(
            "📦 更新完成: {} 新增, {} 更新, {} 失败",
            result.added, result.updated, result.failed
        );
    }

    // 路由
    let app = Router::new()
        // 核心路由
        .route("/", get(index_handler).post(search_handler))
        .route("/api", get(api_info_handler))
        .route("/rules", get(rules_handler))
        .route("/update", get(update_handler))
        .route("/health", get(health_handler))
        // Bangumi 公开 API
        .route("/bangumi/search/{keyword}", get(bangumi_search_handler))
        .route("/bangumi/subject/{id}", get(bangumi_subject_handler))
        .route("/bangumi/calendar", get(bangumi_calendar_handler))
        // Bangumi v0 条目 API
        .route("/bangumi/v0/search", post(bangumi_v0_search_handler))
        .route("/bangumi/v0/subjects/{id}", get(bangumi_v0_subject_handler))
        .route("/bangumi/v0/subjects/{id}/characters", get(bangumi_subject_characters_handler))
        .route("/bangumi/v0/subjects/{id}/persons", get(bangumi_subject_persons_handler))
        .route("/bangumi/v0/subjects/{id}/subjects", get(bangumi_subject_relations_handler))
        // Bangumi 章节 API
        .route("/bangumi/v0/episodes", get(bangumi_episodes_handler))
        .route("/bangumi/v0/episodes/{id}", get(bangumi_episode_handler))
        // Bangumi 角色/人物 API
        .route("/bangumi/v0/characters/{id}", get(bangumi_character_handler))
        .route("/bangumi/v0/characters/{id}/collect", post(bangumi_collect_character_handler).delete(bangumi_uncollect_character_handler))
        .route("/bangumi/v0/persons/{id}", get(bangumi_person_handler))
        .route("/bangumi/v0/persons/{id}/collect", post(bangumi_collect_person_handler).delete(bangumi_uncollect_person_handler))
        // Bangumi 用户 API
        .route("/bangumi/v0/users/{username}", get(bangumi_user_handler))
        .route("/bangumi/v0/me", get(bangumi_me_handler))
        // Bangumi 收藏 API
        .route("/bangumi/v0/users/{username}/collections", get(bangumi_user_collections_handler))
        .route("/bangumi/v0/users/{username}/collections/{subject_id}", get(bangumi_user_collection_handler))
        .route("/bangumi/v0/collections/{subject_id}", post(bangumi_add_collection_handler).patch(bangumi_update_collection_handler))
        .route("/bangumi/v0/collections/{subject_id}/episodes", get(bangumi_episode_collections_handler))
        .route("/bangumi/v0/collections/episodes/{episode_id}", put(bangumi_update_episode_collection_handler))
        // Bangumi 目录 API
        .route("/bangumi/v0/indices/{id}", get(bangumi_index_handler))
        .route("/bangumi/v0/indices/{id}/subjects", get(bangumi_index_subjects_handler))
        .route("/bangumi/v0/indices/{id}/collect", post(bangumi_collect_index_handler).delete(bangumi_uncollect_index_handler))
        .layer(cors);

    // 启动服务器
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("🚀 动漫聚搜 API 启动在 http://{}", addr);
    info!("📚 已加载 {} 个规则", get_builtin_rules().len());

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// GET / - 最小前端页面
async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// GET /api - API 信息
async fn api_info_handler() -> impl IntoResponse {
    Json(json!({
        "name": "AnimeSearch API",
        "version": "0.2.0",
        "description": "在线动漫聚合搜索后端 (支持 Bangumi API)",
        "endpoints": {
            "core": {
                "GET /": "搜索页面",
                "POST /": "搜索动漫 (FormData: anime=关键词, rules=规则名1,规则名2)",
                "GET /rules": "获取所有规则列表",
                "GET /update": "从 KazumiRules 更新规则",
                "GET /health": "健康检查"
            },
            "bangumi_public": {
                "GET /bangumi/search/{keyword}": "搜索动漫",
                "GET /bangumi/subject/{id}": "获取条目详情",
                "GET /bangumi/calendar": "每日放送"
            },
            "bangumi_v0": {
                "POST /bangumi/v0/search": "v0 条目搜索 (JSON)",
                "GET /bangumi/v0/subjects/{id}": "获取条目详情 v0",
                "GET /bangumi/v0/subjects/{id}/characters": "获取条目角色",
                "GET /bangumi/v0/subjects/{id}/persons": "获取条目制作人员",
                "GET /bangumi/v0/subjects/{id}/subjects": "获取关联条目",
                "GET /bangumi/v0/episodes": "获取章节列表 (?subject_id=)",
                "GET /bangumi/v0/episodes/{id}": "获取章节详情",
                "GET /bangumi/v0/characters/{id}": "获取角色详情",
                "POST /bangumi/v0/characters/{id}/collect": "收藏角色 🔐",
                "DELETE /bangumi/v0/characters/{id}/collect": "取消收藏角色 🔐",
                "GET /bangumi/v0/persons/{id}": "获取人物详情",
                "POST /bangumi/v0/persons/{id}/collect": "收藏人物 🔐",
                "DELETE /bangumi/v0/persons/{id}/collect": "取消收藏人物 🔐",
                "GET /bangumi/v0/users/{username}": "获取用户信息",
                "GET /bangumi/v0/me": "获取当前用户 🔐",
                "GET /bangumi/v0/users/{username}/collections": "获取用户收藏 🔐",
                "GET /bangumi/v0/users/{username}/collections/{subject_id}": "获取单个收藏 🔐",
                "POST /bangumi/v0/collections/{subject_id}": "添加收藏 🔐",
                "PATCH /bangumi/v0/collections/{subject_id}": "修改收藏 🔐",
                "GET /bangumi/v0/collections/{subject_id}/episodes": "章节收藏信息 🔐",
                "PUT /bangumi/v0/collections/episodes/{episode_id}": "更新章节收藏 🔐",
                "GET /bangumi/v0/indices/{id}": "获取目录详情",
                "GET /bangumi/v0/indices/{id}/subjects": "获取目录条目",
                "POST /bangumi/v0/indices/{id}/collect": "收藏目录 🔐",
                "DELETE /bangumi/v0/indices/{id}/collect": "取消收藏目录 🔐"
            }
        },
        "auth": {
            "note": "🔐 标记的端点需要 Authorization: Bearer <token> 请求头",
            "get_token": "https://next.bgm.tv/demo/access-token"
        }
    }))
}

/// POST / - 动漫搜索处理器 (SSE 流式响应)
async fn search_handler(mut multipart: Multipart) -> Response {
    // 解析 FormData
    let mut keyword: Option<String> = None;
    let mut rule_names: Option<String> = None;
    let mut fetch_episodes = false;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("anime") => {
                if let Ok(text) = field.text().await {
                    keyword = Some(text.trim().to_string());
                }
            }
            Some("rules") => {
                if let Ok(text) = field.text().await {
                    rule_names = Some(text.trim().to_string());
                }
            }
            Some("episodes") => {
                if let Ok(text) = field.text().await {
                    fetch_episodes = text.trim() == "1" || text.trim().to_lowercase() == "true";
                }
            }
            _ => {}
        }
    }

    let keyword = match keyword {
        Some(k) if !k.is_empty() => k,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({"error": "Anime name is required"})),
            )
                .into_response();
        }
    };

    // 筛选规则
    let all_rules = get_builtin_rules();
    let selected_rules: Vec<_> = match rule_names {
        Some(names) if !names.is_empty() => {
            let name_list: Vec<&str> = names.split(',').map(|s| s.trim()).collect();
            all_rules
                .into_iter()
                .filter(|r| name_list.contains(&r.name.as_str()))
                .collect()
        }
        _ => {
            // 如果没有指定规则，返回错误
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({"error": "Rules are required. Use 'rules' field to specify rule names (comma separated)"})),
            )
                .into_response();
        }
    };

    if selected_rules.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!({"error": "No matching rules found"})),
        )
            .into_response();
    }

    info!(
        "🔍 搜索: {} (规则: {}, 获取集数: {})",
        keyword,
        selected_rules
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        fetch_episodes
    );

    // 创建 SSE 流
    let stream = search_stream_with_rules_options(keyword, selected_rules, fetch_episodes);

    // 将流转换为字节流
    let body = Body::from_stream(stream.map(|s| Ok::<_, std::convert::Infallible>(s)));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(body)
        .unwrap()
}

/// 获取规则列表
async fn rules_handler() -> impl IntoResponse {
    let rules = get_builtin_rules();
    let rule_info: Vec<_> = rules
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "version": r.version,
                "baseUrl": r.base_url,
                "color": r.color,
                "tags": r.tags,
                "magic": r.magic
            })
        })
        .collect();

    Json(rule_info)
}

/// 健康检查
async fn health_handler() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// GET /update - 从 KazumiRules 更新规则
async fn update_handler() -> impl IntoResponse {
    info!("📡 手动触发规则更新...");
    let result = updater::update_rules().await;
    Json(json!({
        "success": true,
        "total": result.total,
        "added": result.added,
        "updated": result.updated,
        "failed": result.failed,
        "details": result.details
    }))
}

/// GET /bangumi/search/{keyword} - Bangumi 搜索
async fn bangumi_search_handler(
    axum::extract::Path(keyword): axum::extract::Path<String>,
) -> impl IntoResponse {
    let results = bangumi::search_anime_simple(&keyword).await;
    Json(results)
}

/// GET /bangumi/subject/{id} - 获取 Bangumi 条目详情
async fn bangumi_subject_handler(
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    match bangumi::get_subject(id).await {
        Ok(subject) => Json(json!(subject)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/calendar - 每日放送
async fn bangumi_calendar_handler() -> impl IntoResponse {
    match bangumi::get_calendar().await {
        Ok(calendar) => Json(json!(calendar)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ============================================================================
// Bangumi v0 API 处理函数
// ============================================================================

/// 从请求头提取 Bearer Token (如果用户未提供则使用服务端默认 token)
fn extract_token(headers: &HeaderMap) -> Option<String> {
    let user_token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .filter(|s| !s.is_empty());
    
    bangumi::get_effective_token(user_token).map(|s| s.to_string())
}

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CollectionQuery {
    pub subject_type: Option<i32>,
    #[serde(rename = "type")]
    pub collection_type: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodeQuery {
    pub subject_id: i64,
    #[serde(rename = "type")]
    pub episode_type: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodeCollectionQuery {
    pub episode_type: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// v0 搜索请求体
#[derive(Debug, Deserialize)]
pub struct V0SearchRequest {
    pub keyword: String,
    #[serde(default)]
    pub filter: Option<V0SearchFilter>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct V0SearchFilter {
    #[serde(rename = "type")]
    pub subject_type: Option<Vec<i32>>,
    pub tag: Option<Vec<String>>,
    pub air_date: Option<Vec<String>>,
    pub rating: Option<Vec<String>>,
    pub rank: Option<Vec<String>>,
    pub nsfw: Option<bool>,
}

/// POST /bangumi/v0/search - v0 条目搜索
async fn bangumi_v0_search_handler(
    headers: HeaderMap,
    Json(req): Json<V0SearchRequest>,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    let search_req = bangumi::SearchRequest {
        keyword: req.keyword,
        filter: req.filter.map(|f| bangumi::SearchFilter {
            subject_type: f.subject_type,
            tag: f.tag,
            air_date: f.air_date,
            rating: f.rating,
            rank: f.rank,
            nsfw: f.nsfw,
        }),
    };

    match bangumi::search_subjects_v0(&search_req, req.limit, req.offset, token.as_deref()).await {
        Ok(result) => Json(json!(result)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/subjects/{id} - 获取条目详情 v0
async fn bangumi_v0_subject_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_subject_v0(id, token.as_deref()).await {
        Ok(subject) => Json(json!(subject)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/subjects/{id}/characters - 获取条目角色
async fn bangumi_subject_characters_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_subject_characters(id, token.as_deref()).await {
        Ok(chars) => Json(json!(chars)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/subjects/{id}/persons - 获取条目制作人员
async fn bangumi_subject_persons_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_subject_persons(id, token.as_deref()).await {
        Ok(persons) => Json(json!(persons)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/subjects/{id}/subjects - 获取关联条目
async fn bangumi_subject_relations_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_subject_relations(id, token.as_deref()).await {
        Ok(relations) => Json(json!(relations)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/episodes - 获取章节列表
async fn bangumi_episodes_handler(
    Query(params): Query<EpisodeQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_episodes(
        params.subject_id,
        params.episode_type,
        params.limit,
        params.offset,
        token.as_deref(),
    )
    .await
    {
        Ok(episodes) => Json(json!(episodes)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/episodes/{id} - 获取章节详情
async fn bangumi_episode_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_episode(id, token.as_deref()).await {
        Ok(episode) => Json(json!(episode)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/characters/{id} - 获取角色详情
async fn bangumi_character_handler(Path(id): Path<i64>) -> impl IntoResponse {
    match bangumi::get_character(id).await {
        Ok(character) => Json(json!(character)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /bangumi/v0/characters/{id}/collect - 收藏角色
async fn bangumi_collect_character_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::collect_character(id, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /bangumi/v0/characters/{id}/collect - 取消收藏角色
async fn bangumi_uncollect_character_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::uncollect_character(id, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/persons/{id} - 获取人物详情
async fn bangumi_person_handler(Path(id): Path<i64>) -> impl IntoResponse {
    match bangumi::get_person(id).await {
        Ok(person) => Json(json!(person)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /bangumi/v0/persons/{id}/collect - 收藏人物
async fn bangumi_collect_person_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::collect_person(id, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /bangumi/v0/persons/{id}/collect - 取消收藏人物
async fn bangumi_uncollect_person_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::uncollect_person(id, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/users/{username} - 获取用户信息
async fn bangumi_user_handler(Path(username): Path<String>) -> impl IntoResponse {
    match bangumi::get_user(&username).await {
        Ok(user) => Json(json!(user)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/me - 获取当前用户信息
async fn bangumi_me_handler(headers: HeaderMap) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::get_me(&token).await {
        Ok(user) => Json(json!(user)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/users/{username}/collections - 获取用户收藏列表
async fn bangumi_user_collections_handler(
    Path(username): Path<String>,
    Query(params): Query<CollectionQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::get_user_collections(
        &username,
        params.subject_type,
        params.collection_type,
        params.limit,
        params.offset,
        &token,
    )
    .await
    {
        Ok(collections) => Json(json!(collections)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/users/{username}/collections/{subject_id} - 获取用户单个条目收藏
async fn bangumi_user_collection_handler(
    Path((username, subject_id)): Path<(String, i64)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::get_user_collection(&username, subject_id, &token).await {
        Ok(collection) => Json(json!(collection)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// 添加收藏请求体
#[derive(Debug, Deserialize)]
pub struct AddCollectionRequest {
    #[serde(rename = "type")]
    pub collection_type: i32,
    pub rate: Option<i32>,
    pub comment: Option<String>,
    pub private: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// POST /bangumi/v0/collections/{subject_id} - 添加收藏
async fn bangumi_add_collection_handler(
    Path(subject_id): Path<i64>,
    headers: HeaderMap,
    Json(req): Json<AddCollectionRequest>,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::add_collection(
        subject_id,
        req.collection_type,
        req.rate,
        req.comment,
        req.private,
        req.tags,
        &token,
    )
    .await
    {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// 修改收藏请求体
#[derive(Debug, Deserialize)]
pub struct UpdateCollectionRequest {
    #[serde(rename = "type")]
    pub collection_type: Option<i32>,
    pub rate: Option<i32>,
    pub ep_status: Option<i32>,
    pub vol_status: Option<i32>,
    pub comment: Option<String>,
    pub private: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// PATCH /bangumi/v0/collections/{subject_id} - 修改收藏
async fn bangumi_update_collection_handler(
    Path(subject_id): Path<i64>,
    headers: HeaderMap,
    Json(req): Json<UpdateCollectionRequest>,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    let modify = bangumi::CollectionModify {
        collection_type: req.collection_type,
        rate: req.rate,
        ep_status: req.ep_status,
        vol_status: req.vol_status,
        comment: req.comment,
        private: req.private,
        tags: req.tags,
    };

    match bangumi::update_collection(subject_id, &modify, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/collections/{subject_id}/episodes - 章节收藏信息
async fn bangumi_episode_collections_handler(
    Path(subject_id): Path<i64>,
    Query(params): Query<EpisodeCollectionQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::get_episode_collections(
        subject_id,
        params.episode_type,
        params.limit,
        params.offset,
        &token,
    )
    .await
    {
        Ok(data) => Json(data).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// 更新章节收藏请求体
#[derive(Debug, Deserialize)]
pub struct UpdateEpisodeCollectionRequest {
    #[serde(rename = "type")]
    pub collection_type: i32,
}

/// PUT /bangumi/v0/collections/episodes/{episode_id} - 更新章节收藏
async fn bangumi_update_episode_collection_handler(
    Path(episode_id): Path<i64>,
    headers: HeaderMap,
    Json(req): Json<UpdateEpisodeCollectionRequest>,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::update_episode_collection(episode_id, req.collection_type, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/indices/{id} - 获取目录详情
async fn bangumi_index_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_index(id, token.as_deref()).await {
        Ok(index) => Json(json!(index)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /bangumi/v0/indices/{id}/subjects - 获取目录条目
async fn bangumi_index_subjects_handler(
    Path(id): Path<i64>,
    Query(params): Query<PaginationQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_token(&headers);
    match bangumi::get_index_subjects(id, params.limit, params.offset, token.as_deref()).await {
        Ok(subjects) => Json(json!(subjects)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /bangumi/v0/indices/{id}/collect - 收藏目录
async fn bangumi_collect_index_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::collect_index(id, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /bangumi/v0/indices/{id}/collect - 取消收藏目录
async fn bangumi_uncollect_index_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authorization token required"})),
            )
                .into_response()
        }
    };

    match bangumi::uncollect_index(id, &token).await {
        Ok(_) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// 最小前端 HTML
const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>动漫聚搜</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
      min-height: 100vh;
      color: #e8e8e8;
      padding: 20px;
    }
    .container { max-width: 1000px; margin: 0 auto; }
    h1 {
      text-align: center;
      font-size: 2rem;
      margin: 30px 0 20px;
      background: linear-gradient(90deg, #ff6b9d, #c44dff);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }
    .search-box {
      display: flex;
      gap: 10px;
      margin-bottom: 16px;
    }
    input[type="text"] {
      flex: 1;
      padding: 14px 18px;
      border: none;
      border-radius: 12px;
      background: rgba(255,255,255,0.1);
      color: #fff;
      font-size: 16px;
      outline: none;
      backdrop-filter: blur(10px);
    }
    input::placeholder { color: rgba(255,255,255,0.5); }
    input:focus { background: rgba(255,255,255,0.15); }
    button {
      padding: 14px 28px;
      border: none;
      border-radius: 12px;
      background: linear-gradient(135deg, #ff6b9d, #c44dff);
      color: #fff;
      font-size: 16px;
      font-weight: 600;
      cursor: pointer;
      transition: transform 0.2s, opacity 0.2s;
    }
    button:hover { transform: scale(1.02); }
    button:active { transform: scale(0.98); }
    button:disabled { opacity: 0.6; cursor: not-allowed; }
    
    .rules-section { margin-bottom: 20px; }
    .rules-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 10px;
    }
    .rules-header span { font-size: 14px; color: rgba(255,255,255,0.7); }
    .rules-actions { display: flex; gap: 8px; }
    .rules-actions button {
      padding: 6px 12px;
      font-size: 12px;
      background: rgba(255,255,255,0.1);
    }
    .rules-grid {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      max-height: 120px;
      overflow-y: auto;
    }
    .rule-tag {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 8px 12px;
      background: rgba(255,255,255,0.08);
      border-radius: 8px;
      cursor: pointer;
      transition: background 0.2s;
      font-size: 13px;
    }
    .rule-tag:hover { background: rgba(255,255,255,0.12); }
    .rule-tag.selected { background: rgba(196, 77, 255, 0.3); }
    .rule-tag input { display: none; }
    .rule-dot { width: 8px; height: 8px; border-radius: 50%; }
    
    /* Bangumi 信息卡片 */
    .bangumi-section {
      margin-bottom: 20px;
      display: none;
    }
    .bangumi-card {
      display: flex;
      gap: 16px;
      background: rgba(255,255,255,0.05);
      border-radius: 12px;
      padding: 16px;
      backdrop-filter: blur(10px);
    }
    .bangumi-cover {
      width: 100px;
      height: 140px;
      border-radius: 8px;
      object-fit: cover;
      flex-shrink: 0;
    }
    .bangumi-info { flex: 1; min-width: 0; }
    .bangumi-title {
      font-size: 18px;
      font-weight: 600;
      margin-bottom: 4px;
      color: #fff;
    }
    .bangumi-title-jp {
      font-size: 13px;
      color: rgba(255,255,255,0.5);
      margin-bottom: 8px;
    }
    .bangumi-meta {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      margin-bottom: 8px;
    }
    .bangumi-score {
      display: flex;
      align-items: center;
      gap: 4px;
      background: linear-gradient(135deg, #f97316, #fb923c);
      padding: 4px 10px;
      border-radius: 6px;
      font-weight: 600;
      font-size: 14px;
    }
    .bangumi-rank {
      background: rgba(255,255,255,0.1);
      padding: 4px 10px;
      border-radius: 6px;
      font-size: 13px;
    }
    .bangumi-date {
      background: rgba(255,255,255,0.1);
      padding: 4px 10px;
      border-radius: 6px;
      font-size: 13px;
    }
    .bangumi-summary {
      font-size: 13px;
      color: rgba(255,255,255,0.7);
      line-height: 1.5;
      max-height: 60px;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .bangumi-link {
      display: inline-block;
      margin-top: 8px;
      font-size: 12px;
      color: #c44dff;
      text-decoration: none;
    }
    .bangumi-link:hover { text-decoration: underline; }
    
    .progress {
      background: rgba(255,255,255,0.1);
      border-radius: 8px;
      height: 6px;
      margin-bottom: 20px;
      overflow: hidden;
      display: none;
    }
    .progress-bar {
      height: 100%;
      background: linear-gradient(90deg, #ff6b9d, #c44dff);
      width: 0%;
      transition: width 0.3s;
    }
    .results { display: flex; flex-direction: column; gap: 16px; }
    .platform {
      background: rgba(255,255,255,0.05);
      border-radius: 12px;
      padding: 16px;
      backdrop-filter: blur(10px);
    }
    .platform-header {
      display: flex;
      align-items: center;
      gap: 10px;
      margin-bottom: 12px;
    }
    .platform-name {
      font-weight: 600;
      font-size: 14px;
      padding: 4px 10px;
      border-radius: 6px;
    }
    .platform-count { font-size: 12px; color: rgba(255,255,255,0.6); }
    .items { display: flex; flex-direction: column; gap: 8px; }
    .item {
      display: block;
      padding: 10px 14px;
      background: rgba(255,255,255,0.05);
      border-radius: 8px;
      color: #e8e8e8;
      text-decoration: none;
      font-size: 14px;
      transition: background 0.2s;
      word-break: break-all;
    }
    .item:hover { background: rgba(255,255,255,0.1); }
    .item-header { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
    .item-name { flex: 1; min-width: 0; word-break: break-all; }
    .item-toggle {
      font-size: 12px;
      color: #c44dff;
      cursor: pointer;
      white-space: nowrap;
      padding: 2px 8px;
      border-radius: 4px;
      background: rgba(196, 77, 255, 0.1);
      transition: background 0.2s;
    }
    .item-toggle:hover { background: rgba(196, 77, 255, 0.2); }
    .episodes-panel {
      margin-top: 10px;
      padding-top: 10px;
      border-top: 1px solid rgba(255,255,255,0.1);
      display: none;
    }
    .episodes-panel.show { display: block; }
    .road-name {
      font-size: 12px;
      color: rgba(255,255,255,0.6);
      margin-bottom: 8px;
      padding: 4px 8px;
      background: rgba(255,255,255,0.05);
      border-radius: 4px;
      display: inline-block;
    }
    .episodes-grid {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      margin-bottom: 12px;
    }
    .episode-btn {
      padding: 6px 12px;
      font-size: 12px;
      background: rgba(255,255,255,0.08);
      border-radius: 6px;
      color: #e8e8e8;
      text-decoration: none;
      transition: background 0.2s, transform 0.2s;
    }
    .episode-btn:hover { background: rgba(196, 77, 255, 0.3); transform: scale(1.05); }
    
    .options-row {
      display: flex;
      align-items: center;
      gap: 16px;
      margin-bottom: 16px;
    }
    .option-switch {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 14px;
      color: rgba(255,255,255,0.8);
      cursor: pointer;
    }
    .switch {
      position: relative;
      width: 40px;
      height: 22px;
      background: rgba(255,255,255,0.15);
      border-radius: 11px;
      transition: background 0.3s;
    }
    .switch::before {
      content: '';
      position: absolute;
      top: 2px;
      left: 2px;
      width: 18px;
      height: 18px;
      background: #fff;
      border-radius: 50%;
      transition: transform 0.3s;
    }
    .switch.active {
      background: linear-gradient(135deg, #ff6b9d, #c44dff);
    }
    .switch.active::before {
      transform: translateX(18px);
    }
    
    .error { color: #ff6b6b; font-size: 13px; }
    .empty { color: rgba(255,255,255,0.4); font-size: 14px; text-align: center; padding: 40px; }
    .loading-rules { text-align: center; padding: 20px; color: rgba(255,255,255,0.5); }
  </style>
</head>
<body>
  <div class="container">
    <h1>🎬 动漫聚搜</h1>
    <div class="search-box">
      <input type="text" id="keyword" placeholder="输入动漫名称..." autofocus>
      <button id="searchBtn" onclick="search()">搜索</button>
    </div>
    
    <div class="rules-section">
      <div class="rules-header">
        <span>选择搜索源 (<span id="selectedCount">0</span> 个已选)</span>
        <div class="rules-actions">
          <button onclick="selectAll()">全选</button>
          <button onclick="selectNone()">取消</button>
        </div>
      </div>
      <div class="rules-grid" id="rulesGrid">
        <div class="loading-rules">加载规则中...</div>
      </div>
    </div>
    
    <div class="options-row">
      <label class="option-switch" onclick="toggleEpisodes()">
        <span class="switch" id="episodesSwitch"></span>
        <span>获取集数选择</span>
      </label>
      <span style="font-size:12px;color:rgba(255,255,255,0.4)">(会增加搜索时间)</span>
    </div>
    
    <!-- Bangumi 信息 -->
    <div class="bangumi-section" id="bangumiSection">
      <div class="bangumi-card" id="bangumiCard"></div>
    </div>
    
    <div class="progress" id="progress">
      <div class="progress-bar" id="progressBar"></div>
    </div>
    <div class="results" id="results"></div>
  </div>
  
  <script>
    const input = document.getElementById('keyword');
    const btn = document.getElementById('searchBtn');
    const progress = document.getElementById('progress');
    const progressBar = document.getElementById('progressBar');
    const results = document.getElementById('results');
    const rulesGrid = document.getElementById('rulesGrid');
    const selectedCount = document.getElementById('selectedCount');
    const bangumiSection = document.getElementById('bangumiSection');
    const bangumiCard = document.getElementById('bangumiCard');
    
    const colors = {
      orange: '#f97316', cyan: '#06b6d4', purple: '#a855f7',
      lime: '#84cc16', pink: '#ec4899', teal: '#14b8a6',
      blue: '#3b82f6', rose: '#f43f5e', amber: '#f59e0b',
      red: '#ef4444', white: '#94a3b8', green: '#22c55e',
      yellow: '#eab308', indigo: '#6366f1', sky: '#0ea5e9'
    };
    
    let allRules = [];
    let fetchEpisodes = false;
    const episodesSwitch = document.getElementById('episodesSwitch');

    input.addEventListener('keydown', e => { if (e.key === 'Enter') search(); });
    
    function toggleEpisodes() {
      fetchEpisodes = !fetchEpisodes;
      episodesSwitch.classList.toggle('active', fetchEpisodes);
    }

    async function loadRules() {
      try {
        const res = await fetch('/rules');
        allRules = await res.json();
        renderRules();
      } catch (e) {
        rulesGrid.innerHTML = '<div class="error">加载规则失败</div>';
      }
    }
    
    function renderRules() {
      rulesGrid.innerHTML = allRules.map(rule => `
        <label class="rule-tag" data-name="${rule.name}">
          <input type="checkbox" value="${rule.name}">
          <span class="rule-dot" style="background:${colors[rule.color] || colors.white}"></span>
          ${rule.name}
        </label>
      `).join('');
      
      rulesGrid.querySelectorAll('.rule-tag').forEach(tag => {
        tag.addEventListener('click', () => {
          const cb = tag.querySelector('input');
          cb.checked = !cb.checked;
          tag.classList.toggle('selected', cb.checked);
          updateCount();
        });
      });
    }
    
    function updateCount() {
      selectedCount.textContent = rulesGrid.querySelectorAll('input:checked').length;
    }
    
    function selectAll() {
      rulesGrid.querySelectorAll('.rule-tag').forEach(tag => {
        tag.querySelector('input').checked = true;
        tag.classList.add('selected');
      });
      updateCount();
    }
    
    function selectNone() {
      rulesGrid.querySelectorAll('.rule-tag').forEach(tag => {
        tag.querySelector('input').checked = false;
        tag.classList.remove('selected');
      });
      updateCount();
    }
    
    function getSelectedRules() {
      return Array.from(rulesGrid.querySelectorAll('input:checked')).map(cb => cb.value);
    }
    
    // 获取 Bangumi 信息
    async function fetchBangumiInfo(keyword) {
      try {
        const res = await fetch('/bangumi/search/' + encodeURIComponent(keyword));
        const data = await res.json();
        if (data && data.length > 0) {
          renderBangumiInfo(data[0]);
        } else {
          bangumiSection.style.display = 'none';
        }
      } catch (e) {
        bangumiSection.style.display = 'none';
      }
    }
    
    function renderBangumiInfo(info) {
      bangumiSection.style.display = 'block';
      bangumiCard.innerHTML = `
        <img class="bangumi-cover" src="${info.image || ''}" alt="${info.name_cn || info.name}" onerror="this.style.display='none'">
        <div class="bangumi-info">
          <div class="bangumi-title">${info.name_cn || info.name}</div>
          ${info.name_cn ? `<div class="bangumi-title-jp">${info.name}</div>` : ''}
          <div class="bangumi-meta">
            ${info.score ? `<span class="bangumi-score">⭐ ${info.score.toFixed(1)}</span>` : ''}
            ${info.rank ? `<span class="bangumi-rank">#${info.rank}</span>` : ''}
            ${info.air_date ? `<span class="bangumi-date">📅 ${info.air_date}</span>` : ''}
          </div>
          ${info.summary ? `<div class="bangumi-summary">${info.summary}</div>` : ''}
          <a class="bangumi-link" href="${info.url}" target="_blank" rel="noopener">在 Bangumi 查看详情 →</a>
        </div>
      `;
    }

    async function search() {
      const keyword = input.value.trim();
      const selectedRules = getSelectedRules();
      
      if (!keyword) { alert('请输入动漫名称'); return; }
      if (selectedRules.length === 0) { alert('请至少选择一个搜索源'); return; }

      btn.disabled = true;
      btn.textContent = '搜索中...';
      results.innerHTML = '';
      bangumiSection.style.display = 'none';
      progress.style.display = 'block';
      progressBar.style.width = '0%';
      
      // 同时获取 Bangumi 信息
      fetchBangumiInfo(keyword);

      try {
        const formData = new FormData();
        formData.append('anime', keyword);
        formData.append('rules', selectedRules.join(','));
        if (fetchEpisodes) formData.append('episodes', '1');

        const response = await fetch('/', { method: 'POST', body: formData });
        
        if (!response.ok) {
          const err = await response.json();
          throw new Error(err.error || '请求失败');
        }
        
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split('\n');
          buffer = lines.pop() || '';

          for (const line of lines) {
            if (!line.trim()) continue;
            try {
              const data = JSON.parse(line);
              if (data.progress) {
                progressBar.style.width = (data.progress.completed / data.progress.total * 100) + '%';
              }
              if (data.result) renderPlatform(data.result);
              if (data.done) {
                progress.style.display = 'none';
                if (!results.children.length) {
                  results.innerHTML = '<div class="empty">未找到相关结果</div>';
                }
              }
            } catch {}
          }
        }
      } catch (e) {
        results.innerHTML = '<div class="error">搜索失败: ' + e.message + '</div>';
        progress.style.display = 'none';
      } finally {
        btn.disabled = false;
        btn.textContent = '搜索';
      }
    }

    function renderPlatform(result) {
      if (result.error && !result.items?.length) return;
      
      const color = colors[result.color] || colors.white;
      const div = document.createElement('div');
      div.className = 'platform';
      div.innerHTML = `
        <div class="platform-header">
          <span class="platform-name" style="background:${color}20;color:${color}">${result.name}</span>
          <span class="platform-count">${result.items?.length || 0} 个结果</span>
          ${result.error ? '<span class="error">' + result.error + '</span>' : ''}
        </div>
        <div class="items">
          ${(result.items || []).map((item, idx) => renderItem(item, idx)).join('')}
        </div>
      `;
      results.appendChild(div);
      
      // 绑定集数展开事件
      div.querySelectorAll('.item-toggle').forEach(btn => {
        btn.addEventListener('click', (e) => {
          e.preventDefault();
          e.stopPropagation();
          const panel = btn.closest('.item').querySelector('.episodes-panel');
          if (panel) {
            panel.classList.toggle('show');
            btn.textContent = panel.classList.contains('show') ? '收起' : '展开集数';
          }
        });
      });
    }
    
    function renderItem(item, idx) {
      const hasEpisodes = item.episodes && item.episodes.length > 0;
      const episodesHtml = hasEpisodes ? renderEpisodes(item.episodes) : '';
      
      return `
        <div class="item">
          <div class="item-header">
            <a class="item-name" href="${item.url}" target="_blank" rel="noopener">${item.name}</a>
            ${hasEpisodes ? '<span class="item-toggle">展开集数</span>' : ''}
          </div>
          ${hasEpisodes ? `<div class="episodes-panel">${episodesHtml}</div>` : ''}
        </div>
      `;
    }
    
    function renderEpisodes(roads) {
      return roads.map(road => `
        ${road.name ? `<div class="road-name">${road.name}</div>` : ''}
        <div class="episodes-grid">
          ${road.episodes.map(ep => 
            `<a class="episode-btn" href="${ep.url}" target="_blank" rel="noopener">${ep.name}</a>`
          ).join('')}
        </div>
      `).join('');
    }
    
    loadRules();
  </script>
</body>
</html>"#;
