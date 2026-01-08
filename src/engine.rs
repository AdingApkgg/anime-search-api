//! 规则驱动的搜索引擎
//! 完全兼容 Kazumi 规则格式: https://github.com/Predidit/Kazumi
//! 使用纯 Rust 库 (scraper) 进行 HTML 解析，通过 XPath→CSS 转换支持规则

use crate::http_client::{get_text, post_form_text};
use crate::types::{Episode, EpisodeRoad, PlatformSearchResult, Rule, SearchResultItem};
use crate::xpath_to_css::{xpath_to_css, PositionFilter};
use scraper::{Html, Selector, ElementRef};
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
    let items = parse_search_results(rule, &html)?;
    
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
    parse_episodes(rule, &html, detail_url)
}

/// 解析章节列表
fn parse_episodes(rule: &Rule, html: &str, base_url: &str) -> anyhow::Result<Vec<EpisodeRoad>> {
    let mut roads = Vec::new();
    let document = Html::parse_document(html);

    // 转换 XPath 为 CSS
    let roads_css = xpath_to_css(&rule.chapter_roads)
        .map_err(|e| anyhow::anyhow!("播放源 XPath 转换失败: {}", e))?;
    let result_css = xpath_to_css(&rule.chapter_result)
        .map_err(|e| anyhow::anyhow!("章节 XPath 转换失败: {}", e))?;

    debug!("播放源 CSS: {}", roads_css.selector);
    debug!("章节 CSS: {}", result_css.selector);

    let roads_selector = Selector::parse(&roads_css.selector)
        .map_err(|e| anyhow::anyhow!("无效的播放源 CSS 选择器: {:?}", e))?;
    let result_selector = Selector::parse(&result_css.selector)
        .map_err(|e| anyhow::anyhow!("无效的章节 CSS 选择器: {:?}", e))?;

    // 提取 base_url 用于构建完整 URL
    let url_base = extract_base_url(base_url, &rule.base_url);

    // 查询播放源列表
    let road_elements: Vec<ElementRef> = document.select(&roads_selector)
        .enumerate()
        .filter(|(i, _)| apply_position_filter(*i, &roads_css.position_filter))
        .map(|(_, e)| e)
        .collect();

    debug!("找到 {} 个播放源", road_elements.len());

    for (index, road_element) in road_elements.iter().enumerate() {
        let mut episodes = Vec::new();

        // 在播放源内查找章节
        for ep_element in road_element.select(&result_selector) {
                // 获取集数名称
            let name = get_element_text(&ep_element).trim().to_string();
                
                // 获取播放链接
            let href = ep_element.value().attr("href").unwrap_or_default().to_string();
                
                if name.is_empty() || href.is_empty() {
                    continue;
                }

                let url = normalize_url(&href, &url_base);
                episodes.push(Episode { name, url });
        }

        if !episodes.is_empty() {
            roads.push(EpisodeRoad {
                name: if road_elements.len() > 1 {
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

/// 解析搜索结果 (兼容 Kazumi 规则)
fn parse_search_results(rule: &Rule, html: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    let mut items = Vec::new();
    let document = Html::parse_document(html);

    // 转换 XPath 为 CSS
    let list_css = xpath_to_css(&rule.search_list)
        .map_err(|e| anyhow::anyhow!("列表 XPath 转换失败: {}", e))?;
    let name_css = xpath_to_css(&rule.search_name)
        .map_err(|e| anyhow::anyhow!("名称 XPath 转换失败: {}", e))?;
    let result_css = if rule.search_result.is_empty() {
        name_css.clone()
    } else {
        xpath_to_css(&rule.search_result)
            .map_err(|e| anyhow::anyhow!("结果 XPath 转换失败: {}", e))?
    };

    debug!("列表 CSS: {}", list_css.selector);
    debug!("名称 CSS: {}", name_css.selector);
    debug!("结果 CSS: {}", result_css.selector);

    let list_selector = Selector::parse(&list_css.selector)
        .map_err(|e| anyhow::anyhow!("无效的列表 CSS 选择器: {:?}", e))?;
    let name_selector = Selector::parse(&name_css.selector)
        .map_err(|e| anyhow::anyhow!("无效的名称 CSS 选择器: {:?}", e))?;
    let result_selector = Selector::parse(&result_css.selector)
        .map_err(|e| anyhow::anyhow!("无效的结果 CSS 选择器: {:?}", e))?;

    // 查询列表元素
    let list_elements: Vec<ElementRef> = document.select(&list_selector)
        .enumerate()
        .filter(|(i, _)| apply_position_filter(*i, &list_css.position_filter))
        .map(|(_, e)| e)
        .collect();

    debug!("找到 {} 个列表节点", list_elements.len());

    for element in list_elements {
        // 在列表项内查找名称
        let name = element.select(&name_selector)
            .next()
            .map(|e| get_element_text(&e).trim().to_string())
            .unwrap_or_default();

        // 在列表项内查找链接
        let href = element.select(&result_selector)
            .next()
            .and_then(|e| {
                // 尝试获取 href 属性
                e.value().attr("href")
                    .or_else(|| e.value().attr("data-href"))
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                // 如果没有找到，尝试在元素内查找 a 标签
                let a_selector = Selector::parse("a[href]").ok()?;
                element.select(&a_selector)
                    .next()
                    .and_then(|a| a.value().attr("href").map(|s| s.to_string()))
            })
            .unwrap_or_default();

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

/// 应用位置过滤器
fn apply_position_filter(index: usize, filter: &Option<PositionFilter>) -> bool {
    match filter {
        Some(PositionFilter::GreaterThan(n)) => index >= *n,
        None => true,
        }
    }

/// 获取元素的文本内容
fn get_element_text(element: &ElementRef) -> String {
    element.text().collect::<Vec<_>>().join(" ").trim().to_string()
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
    fn test_parse_html_with_css() {
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

        let document = Html::parse_document(html);
        let selector = Selector::parse("div.item").unwrap();
        let items: Vec<_> = document.select(&selector).collect();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_get_element_text() {
        let html = r#"<div><span>Hello</span> <span>World</span></div>"#;
        let document = Html::parse_document(html);
        let selector = Selector::parse("div").unwrap();
        let element = document.select(&selector).next().unwrap();
        let text = get_element_text(&element);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }
}
