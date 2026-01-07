//! 规则自动更新器
//! 从 KazumiRules 仓库获取最新规则

use crate::http_client::HTTP_CLIENT;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Kazumi 规则仓库地址
const KAZUMI_RULES_INDEX: &str =
    "https://raw.githubusercontent.com/Predidit/KazumiRules/main/index.json";
const KAZUMI_RULES_BASE: &str =
    "https://raw.githubusercontent.com/Predidit/KazumiRules/main/";

/// 规则目录
const RULES_DIR: &str = "rules";

/// 索引项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexItem {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub use_native_player: bool,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub last_update: u64,
}

/// 更新结果
#[derive(Debug, Clone, Serialize)]
pub struct UpdateResult {
    pub total: usize,
    pub updated: usize,
    pub added: usize,
    pub failed: usize,
    pub details: Vec<UpdateDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateDetail {
    pub name: String,
    pub action: String, // "added", "updated", "failed", "skipped"
    pub message: String,
}

/// 从远程获取最新索引
async fn fetch_remote_index() -> anyhow::Result<Vec<IndexItem>> {
    let response = HTTP_CLIENT
        .get(KAZUMI_RULES_INDEX)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("获取远程索引失败: HTTP {}", response.status());
    }

    let index: Vec<IndexItem> = response.json().await?;
    Ok(index)
}

/// 读取本地索引 (从 index.json)
fn read_local_index() -> HashMap<String, IndexItem> {
    let index_path = Path::new(RULES_DIR).join("index.json");
    let mut map = HashMap::new();

    if let Ok(content) = fs::read_to_string(&index_path) {
        if let Ok(items) = serde_json::from_str::<Vec<IndexItem>>(&content) {
            for item in items {
                map.insert(item.name.clone(), item);
            }
        }
    }

    map
}

/// 下载单个规则
async fn download_rule(name: &str) -> anyhow::Result<String> {
    let url = format!("{}{}.json", KAZUMI_RULES_BASE, name);
    let response = HTTP_CLIENT.get(&url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }

    let content = response.text().await?;
    
    // 验证 JSON 格式
    serde_json::from_str::<serde_json::Value>(&content)?;
    
    Ok(content)
}

/// 保存规则到本地
fn save_rule(name: &str, content: &str) -> anyhow::Result<()> {
    let path = Path::new(RULES_DIR).join(format!("{}.json", name));
    fs::write(path, content)?;
    Ok(())
}

/// 保存本地索引
fn save_local_index(items: &[IndexItem]) -> anyhow::Result<()> {
    let index_path = Path::new(RULES_DIR).join("index.json");
    let content = serde_json::to_string_pretty(items)?;
    fs::write(index_path, content)?;
    Ok(())
}

/// 检查并更新规则
pub async fn update_rules() -> UpdateResult {
    let mut result = UpdateResult {
        total: 0,
        updated: 0,
        added: 0,
        failed: 0,
        details: Vec::new(),
    };

    // 获取远程索引
    let remote_index = match fetch_remote_index().await {
        Ok(index) => index,
        Err(e) => {
            warn!("获取远程索引失败: {}", e);
            result.details.push(UpdateDetail {
                name: "index".to_string(),
                action: "failed".to_string(),
                message: format!("获取远程索引失败: {}", e),
            });
            return result;
        }
    };

    result.total = remote_index.len();
    info!("📡 远程索引包含 {} 个规则", remote_index.len());

    // 读取本地索引
    let local_index = read_local_index();

    // 确保规则目录存在
    let _ = fs::create_dir_all(RULES_DIR);

    // 收集更新后的索引项
    let mut updated_index = Vec::new();

    // 检查每个规则
    for remote_item in &remote_index {
        let local_item = local_index.get(&remote_item.name);
        
        let need_update = match local_item {
            None => true, // 本地不存在
            Some(local) => {
                // 版本不同或时间戳更新
                local.version != remote_item.version 
                    || local.last_update < remote_item.last_update
            }
        };

        if need_update {
            match download_rule(&remote_item.name).await {
                Ok(content) => {
                    if let Err(e) = save_rule(&remote_item.name, &content) {
                        warn!("保存规则 {} 失败: {}", remote_item.name, e);
                        result.failed += 1;
                        result.details.push(UpdateDetail {
                            name: remote_item.name.clone(),
                            action: "failed".to_string(),
                            message: format!("保存失败: {}", e),
                        });
                    } else {
                        let action = if local_item.is_some() { "updated" } else { "added" };
                        if local_item.is_some() {
                            result.updated += 1;
                            info!("🔄 更新规则: {} -> v{}", remote_item.name, remote_item.version);
                        } else {
                            result.added += 1;
                            info!("➕ 新增规则: {} v{}", remote_item.name, remote_item.version);
                        }
                        result.details.push(UpdateDetail {
                            name: remote_item.name.clone(),
                            action: action.to_string(),
                            message: format!("v{}", remote_item.version),
                        });
                        updated_index.push(remote_item.clone());
                    }
                }
                Err(e) => {
                    warn!("下载规则 {} 失败: {}", remote_item.name, e);
                    result.failed += 1;
                    result.details.push(UpdateDetail {
                        name: remote_item.name.clone(),
                        action: "failed".to_string(),
                        message: format!("下载失败: {}", e),
                    });
                    // 保留本地版本
                    if let Some(local) = local_item {
                        updated_index.push(local.clone());
                    }
                }
            }
        } else {
            // 无需更新
            updated_index.push(remote_item.clone());
        }
    }

    // 保存更新后的索引
    if let Err(e) = save_local_index(&updated_index) {
        warn!("保存本地索引失败: {}", e);
    }

    info!(
        "✅ 更新完成: {} 新增, {} 更新, {} 失败",
        result.added, result.updated, result.failed
    );

    result
}

