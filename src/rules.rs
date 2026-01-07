//! 规则管理器
//! 从 rules/ 目录读取 JSON 规则文件，兼容 Kazumi 规则格式

use crate::types::Rule;
use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// 规则目录路径
const RULES_DIR: &str = "rules";

/// 全局规则列表
static RULES: Lazy<Vec<Arc<Rule>>> = Lazy::new(load_all_rules);

/// 获取所有规则
pub fn get_builtin_rules() -> Vec<Arc<Rule>> {
    RULES.clone()
}

/// 从 rules/ 目录加载所有规则
fn load_all_rules() -> Vec<Arc<Rule>> {
    let mut rules = Vec::new();
    let rules_path = Path::new(RULES_DIR);

    if !rules_path.exists() {
        warn!("规则目录 {} 不存在，请创建并添加规则文件", RULES_DIR);
        return rules;
    }

    // 读取目录中的所有 JSON 文件
    match fs::read_dir(rules_path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                // 跳过 index.json (Kazumi 索引文件)
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename == "index.json" {
                    continue;
                }
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    match load_rule_from_file(&path) {
                        Ok(rule) => {
                            info!("📦 加载规则: {} v{}", rule.name, rule.version);
                            rules.push(Arc::new(rule));
                        }
                        Err(e) => {
                            warn!("⚠️ 加载规则失败 {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            warn!("读取规则目录失败: {}", e);
        }
    }

    // 按名称排序
    rules.sort_by(|a, b| a.name.cmp(&b.name));

    rules
}

/// 从 JSON 文件加载单个规则
fn load_rule_from_file(path: &Path) -> anyhow::Result<Rule> {
    let content = fs::read_to_string(path)?;
    let rule: Rule = serde_json::from_str(&content)?;
    Ok(rule)
}
