//! 规则驱动的搜索引擎
//! 完全兼容 Kazumi 规则格式: https://github.com/Predidit/Kazumi
//! 使用 libxml 进行真正的 XPath 解析

use crate::http_client::{get_text, post_form_text};
use crate::types::{Episode, EpisodeRoad, PlatformSearchResult, Rule, SearchResultItem};
use libxml::parser::Parser;
use libxml::xpath::Context;
use tracing::{debug, warn};

/// 使用规则搜索动漫
pub async fn search_with_rule(rule: &Rule, keyword: &str) -> PlatformSearchResult {
    match execute_search(rule, keyword).await {
        Ok(items) => PlatformSearchResult::with_items(items),
        Err(e) => {
            warn!("规则 {} 搜索失败: {}", rule.name, e);
            PlatformSearchResult::with_error(e.to_string())
        }
    }
}

/// 使用规则搜索动漫 (包含集数信息)
pub async fn search_with_rule_and_episodes(rule: &Rule, keyword: &str) -> PlatformSearchResult {
    match execute_search_with_episodes(rule, keyword).await {
        Ok(items) => PlatformSearchResult::with_items(items),
        Err(e) => {
            warn!("规则 {} 搜索失败: {}", rule.name, e);
            PlatformSearchResult::with_error(e.to_string())
        }
    }
}

async fn execute_search(rule: &Rule, keyword: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    // 构建搜索 URL
    let search_url = rule.search_url.replace("@keyword", &urlencoding::encode(keyword));
    debug!("搜索 URL: {}", search_url);

    // 发送请求
    let html = if rule.use_post {
        // POST 请求
        let uri = url::Url::parse(&search_url)?;
        let query_params: std::collections::HashMap<String, String> = uri
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let base_url = format!("{}://{}{}", uri.scheme(), uri.host_str().unwrap_or(""), uri.path());
        post_form_text(&base_url, &query_params, Some(&rule.base_url)).await?
    } else {
        // GET 请求
        get_text(&search_url, Some(&rule.base_url)).await?
    };

    // 解析 HTML 并提取结果
    let items = parse_search_results_xpath(rule, &html)?;
    
    debug!("规则 {} 找到 {} 个结果", rule.name, items.len());
    Ok(items)
}

async fn execute_search_with_episodes(rule: &Rule, keyword: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    // 先执行普通搜索
    let mut items = execute_search(rule, keyword).await?;

    // 如果规则有章节选择器，获取每个结果的章节信息
    if !rule.chapter_roads.is_empty() && !rule.chapter_result.is_empty() {
        // 限制并发获取章节的数量，避免请求过多
        let max_items = 5.min(items.len());
        
        for item in items.iter_mut().take(max_items) {
            match fetch_episodes(rule, &item.url).await {
                Ok(episodes) => {
                    if !episodes.is_empty() {
                        item.episodes = Some(episodes);
                    }
                }
                Err(e) => {
                    debug!("获取章节失败 {}: {}", item.url, e);
                }
            }
        }
    }

    Ok(items)
}

/// 获取动漫详情页的章节列表
pub async fn fetch_episodes(rule: &Rule, detail_url: &str) -> anyhow::Result<Vec<EpisodeRoad>> {
    if rule.chapter_roads.is_empty() || rule.chapter_result.is_empty() {
        return Ok(vec![]);
    }

    // 获取详情页 HTML
    let html = get_text(detail_url, Some(&rule.base_url)).await?;
    
    // 解析章节
    parse_episodes_xpath(rule, &html, detail_url)
}

/// 使用 XPath 解析章节列表
fn parse_episodes_xpath(rule: &Rule, html: &str, base_url: &str) -> anyhow::Result<Vec<EpisodeRoad>> {
    let mut roads = Vec::new();

    // 使用 libxml 解析 HTML
    let parser = Parser::default_html();
    let document = parser
        .parse_string(html)
        .map_err(|e| anyhow::anyhow!("HTML 解析失败: {}", e))?;

    // 创建 XPath 上下文
    let context = Context::new(&document)
        .map_err(|_| anyhow::anyhow!("创建 XPath 上下文失败"))?;

    // 规范化 XPath 表达式
    let roads_xpath = normalize_xpath(&rule.chapter_roads);
    let result_xpath = normalize_xpath(&rule.chapter_result);

    debug!("播放源 XPath: {}", roads_xpath);
    debug!("章节 XPath: {}", result_xpath);

    // 查询播放源列表
    let roads_nodes = context.evaluate(&roads_xpath)
        .map_err(|_| anyhow::anyhow!("无效的播放源 XPath: {}", roads_xpath))?;

    let road_nodes = roads_nodes.get_nodes_as_vec();
    debug!("找到 {} 个播放源", road_nodes.len());

    // 提取 base_url 用于构建完整 URL
    let url_base = extract_base_url(base_url, &rule.base_url);

    for (index, road_node) in road_nodes.iter().enumerate() {
        let mut episodes = Vec::new();

        // 为每个播放源创建子上下文
        let mut node_context = Context::new(&document)
            .map_err(|_| anyhow::anyhow!("创建子节点上下文失败"))?;

        // 设置上下文节点
        if node_context.set_context_node(road_node).is_err() {
            continue;
        }

        // 构建相对 XPath
        let relative_xpath = if result_xpath.starts_with("//") {
            format!(".{}", result_xpath)
        } else if result_xpath.starts_with("./") || result_xpath.starts_with(".//") {
            result_xpath.clone()
        } else {
            format!(".//{}", result_xpath)
        };

        // 查询章节列表
        if let Ok(eps_result) = node_context.evaluate(&relative_xpath) {
            let ep_nodes = eps_result.get_nodes_as_vec();
            
            for ep_node in ep_nodes {
                // 获取集数名称
                let name = ep_node.get_content().trim().to_string();
                
                // 获取播放链接
                let href = ep_node.get_attribute("href").unwrap_or_default();
                
                if name.is_empty() || href.is_empty() {
                    continue;
                }

                let url = normalize_url(&href, &url_base);
                episodes.push(Episode { name, url });
            }
        }

        if !episodes.is_empty() {
            roads.push(EpisodeRoad {
                name: if road_nodes.len() > 1 {
                    Some(format!("线路{}", index + 1))
                } else {
                    None
                },
                episodes,
            });
        }
    }

    Ok(roads)
}

/// 使用 XPath 解析搜索结果 (兼容 Kazumi 规则)
fn parse_search_results_xpath(rule: &Rule, html: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    let mut items = Vec::new();

    // 使用 libxml 解析 HTML
    let parser = Parser::default_html();
    let document = parser
        .parse_string(html)
        .map_err(|e| anyhow::anyhow!("HTML 解析失败: {}", e))?;

    // 创建 XPath 上下文
    let context = Context::new(&document)
        .map_err(|_| anyhow::anyhow!("创建 XPath 上下文失败"))?;

    // 规范化 XPath 表达式
    let list_xpath = normalize_xpath(&rule.search_list);
    let name_xpath = normalize_xpath(&rule.search_name);
    let result_xpath = normalize_xpath(&rule.search_result);

    debug!("列表 XPath: {}", list_xpath);
    debug!("名称 XPath: {}", name_xpath);
    debug!("结果 XPath: {}", result_xpath);

    // 查询列表元素
    let list_nodes = context.evaluate(&list_xpath)
        .map_err(|_| anyhow::anyhow!("无效的列表 XPath: {}", list_xpath))?;

    let nodes = list_nodes.get_nodes_as_vec();
    debug!("找到 {} 个列表节点", nodes.len());

    for node in nodes {
        // 为每个列表项创建子上下文
        let mut node_context = Context::new(&document)
            .map_err(|_| anyhow::anyhow!("创建子节点上下文失败"))?;

        // 在当前节点下查询名称
        let name = extract_text_from_node(&mut node_context, &node, &name_xpath);
        
        // 在当前节点下查询链接
        let href = extract_href_from_node(&mut node_context, &node, &result_xpath);

        if name.is_empty() || href.is_empty() {
            continue;
        }

        // 构建完整 URL
        let url = normalize_url(&href, &rule.base_url);

        items.push(SearchResultItem {
            name,
            url,
            tags: None,
            episodes: None,
        });
    }

    Ok(items)
}

/// 规范化 XPath 表达式
fn normalize_xpath(xpath: &str) -> String {
    let xpath = xpath.trim();
    
    // 确保以 // 或 . 开头
    if xpath.starts_with("//") || xpath.starts_with("./") || xpath.starts_with(".//") {
        xpath.to_string()
    } else if xpath.starts_with("/") {
        format!("/{}", xpath)
    } else if !xpath.is_empty() {
        // 对于相对路径，添加 .// 前缀
        format!(".//{}", xpath)
    } else {
        xpath.to_string()
    }
}

/// 从节点中提取文本内容
fn extract_text_from_node(context: &mut Context, node: &libxml::tree::Node, xpath: &str) -> String {
    // 构建相对 XPath
    let relative_xpath = if xpath.starts_with("//") {
        // 绝对路径转相对路径
        format!(".{}", xpath)
    } else if xpath.starts_with("./") || xpath.starts_with(".//") {
        xpath.to_string()
    } else {
        format!(".//{}", xpath)
    };

    // 设置上下文节点
    if context.set_context_node(node).is_err() {
        return String::new();
    }

    // 执行 XPath 查询
    if let Ok(result) = context.evaluate(&relative_xpath) {
        let nodes = result.get_nodes_as_vec();
        if let Some(target_node) = nodes.first() {
            // 获取文本内容
            return get_node_text(target_node);
        }
    }

    // 如果 XPath 查询失败，尝试从当前节点获取文本
    get_node_text(node)
}

/// 从节点中提取 href 属性
fn extract_href_from_node(context: &mut Context, node: &libxml::tree::Node, xpath: &str) -> String {
    // 构建相对 XPath
    let relative_xpath = if xpath.starts_with("//") {
        format!(".{}", xpath)
    } else if xpath.starts_with("./") || xpath.starts_with(".//") {
        xpath.to_string()
    } else {
        format!(".//{}", xpath)
    };

    // 设置上下文节点
    if context.set_context_node(node).is_err() {
        return String::new();
    }

    // 执行 XPath 查询
    if let Ok(result) = context.evaluate(&relative_xpath) {
        let nodes = result.get_nodes_as_vec();
        if let Some(target_node) = nodes.first() {
            // 尝试获取 href 属性
            if let Some(href) = target_node.get_attribute("href") {
                return href;
            }
            // 如果没有 href，尝试获取 data-href 或其他常见属性
            if let Some(href) = target_node.get_attribute("data-href") {
                return href;
            }
            // 递归查找 a 标签
            if let Ok(a_result) = context.evaluate(".//a/@href") {
                let a_nodes = a_result.get_nodes_as_vec();
                if let Some(a_node) = a_nodes.first() {
                    return a_node.get_content();
                }
            }
        }
    }

    String::new()
}

/// 获取节点的文本内容
fn get_node_text(node: &libxml::tree::Node) -> String {
    node.get_content().trim().to_string()
}

/// 规范化 URL
fn normalize_url(href: &str, base_url: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with("//") {
        format!("https:{}", href)
    } else if href.starts_with("/") {
        format!("{}{}", base_url.trim_end_matches('/'), href)
    } else {
        format!("{}/{}", base_url.trim_end_matches('/'), href)
    }
}

/// 从详情页 URL 提取基础 URL
fn extract_base_url(detail_url: &str, rule_base_url: &str) -> String {
    if let Ok(url) = url::Url::parse(detail_url) {
        format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
    } else {
        rule_base_url.trim_end_matches('/').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_xpath() {
        assert_eq!(normalize_xpath("//div/a"), "//div/a");
        assert_eq!(normalize_xpath("./div/a"), "./div/a");
        assert_eq!(normalize_xpath("div/a"), ".//div/a");
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("/video/123", "https://example.com"),
            "https://example.com/video/123"
        );
        assert_eq!(
            normalize_url("//cdn.example.com/img.jpg", "https://example.com"),
            "https://cdn.example.com/img.jpg"
        );
        assert_eq!(
            normalize_url("https://other.com/page", "https://example.com"),
            "https://other.com/page"
        );
    }

    #[test]
    fn test_parse_html_with_xpath() {
        let html = r#"
        <html>
        <body>
            <div class="search-box">
                <div class="item">
                    <h3><a href="/video/1">动漫1</a></h3>
                </div>
                <div class="item">
                    <h3><a href="/video/2">动漫2</a></h3>
                </div>
            </div>
        </body>
        </html>
        "#;

        let parser = Parser::default_html();
        let doc = parser.parse_string(html).unwrap();
        let context = Context::new(&doc).unwrap();

        let result = context.evaluate("//div[@class='item']").unwrap();
        let nodes = result.get_nodes_as_vec();
        assert_eq!(nodes.len(), 2);
    }
}
