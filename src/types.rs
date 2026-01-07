use serde::{Deserialize, Serialize};

/// Kazumi 风格的规则定义
/// 完全兼容 Kazumi 规则格式: https://github.com/Predidit/KazumiRules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// API 版本
    #[serde(default = "default_api")]
    pub api: String,

    /// 类型 (anime)
    #[serde(rename = "type", default = "default_type")]
    pub rule_type: String,

    /// 平台名称
    pub name: String,

    /// 规则版本
    #[serde(default = "default_version")]
    pub version: String,

    /// 是否支持多播放源
    #[serde(default, alias = "muliSources")]
    pub muli_sources: bool,

    /// 是否使用 webview
    #[serde(default, alias = "useWebview")]
    pub use_webview: bool,

    /// 是否使用原生播放器
    #[serde(default = "default_true", alias = "useNativePlayer")]
    pub use_native_player: bool,

    /// 是否使用 POST 请求
    #[serde(default, alias = "usePost")]
    pub use_post: bool,

    /// 是否使用旧版解析器
    #[serde(default, alias = "useLegacyParser")]
    pub use_legacy_parser: bool,

    /// 是否启用广告拦截
    #[serde(default, alias = "adBlocker")]
    pub ad_blocker: bool,

    /// 自定义 User-Agent
    #[serde(default, alias = "userAgent")]
    pub user_agent: String,

    /// 基础 URL
    #[serde(alias = "baseURL")]
    pub base_url: String,

    /// 搜索 URL (使用 @keyword 作为占位符)
    #[serde(alias = "searchURL")]
    pub search_url: String,

    /// 搜索结果列表选择器 (CSS/XPath)
    #[serde(default, alias = "searchList")]
    pub search_list: String,

    /// 搜索结果名称选择器
    #[serde(default, alias = "searchName")]
    pub search_name: String,

    /// 搜索结果链接选择器
    #[serde(default, alias = "searchResult")]
    pub search_result: String,

    /// 章节列表选择器
    #[serde(default, alias = "chapterRoads")]
    pub chapter_roads: String,

    /// 章节结果选择器
    #[serde(default, alias = "chapterResult")]
    pub chapter_result: String,

    /// Referer 头
    #[serde(default)]
    pub referer: String,

    // ========== 扩展字段 (Kazumi 原生不包含) ==========
    
    /// 平台颜色 (用于前端显示)
    #[serde(default = "default_color")]
    pub color: String,

    /// 平台标签 (如：在线, Magnet, BT 等)
    #[serde(default)]
    pub tags: Vec<String>,

    /// 是否需要魔法
    #[serde(default)]
    pub magic: bool,
}

fn default_api() -> String {
    "1".to_string()
}

fn default_type() -> String {
    "anime".to_string()
}

fn default_version() -> String {
    "1.0".to_string()
}

fn default_color() -> String {
    "white".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            api: default_api(),
            rule_type: default_type(),
            name: String::new(),
            version: default_version(),
            muli_sources: false,
            use_webview: false,
            use_native_player: true,
            use_post: false,
            use_legacy_parser: false,
            ad_blocker: false,
            user_agent: String::new(),
            base_url: String::new(),
            search_url: String::new(),
            search_list: String::new(),
            search_name: String::new(),
            search_result: String::new(),
            chapter_roads: String::new(),
            chapter_result: String::new(),
            referer: String::new(),
            color: default_color(),
            tags: vec![],
            magic: false,
        }
    }
}

/// 单个搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// 动漫名称
    pub name: String,
    /// 资源链接
    pub url: String,
    /// 可选标签 (如：集数、画质等)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// 集数列表 (播放源 -> 集数列表)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episodes: Option<Vec<EpisodeRoad>>,
}

/// 播放源 (一个动漫可能有多个播放源)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRoad {
    /// 播放源名称 (如: "线路1", "备用线路")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 该播放源下的集数列表
    pub episodes: Vec<Episode>,
}

/// 单集信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// 集数名称 (如: "第1集", "01")
    pub name: String,
    /// 播放链接
    pub url: String,
}

/// 平台搜索的返回值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSearchResult {
    /// 搜索结果列表
    pub items: Vec<SearchResultItem>,
    /// 结果数量 (-1 表示出错)
    pub count: i32,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PlatformSearchResult {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            count: 0,
            error: None,
        }
    }

    pub fn with_error(message: String) -> Self {
        Self {
            items: Vec::new(),
            count: -1,
            error: Some(message),
        }
    }

    pub fn with_items(items: Vec<SearchResultItem>) -> Self {
        let count = items.len() as i32;
        Self {
            items,
            count,
            error: None,
        }
    }
}

impl Default for PlatformSearchResult {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE 流中的进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProgress {
    /// 已完成的平台数
    pub completed: usize,
    /// 总平台数
    pub total: usize,
}

/// SSE 流中的单个结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResult {
    /// 平台名称
    pub name: String,
    /// 平台颜色
    pub color: String,
    /// 平台标签
    pub tags: Vec<String>,
    /// 搜索结果
    pub items: Vec<SearchResultItem>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// SSE 事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StreamEvent {
    /// 初始事件，包含总数
    Init { total: usize },
    /// 进度更新 (无结果)
    Progress { progress: StreamProgress },
    /// 进度更新 + 结果
    Result {
        progress: StreamProgress,
        result: StreamResult,
    },
    /// 完成信号
    Done { done: bool },
}
