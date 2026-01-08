//! XPath 到 CSS 选择器转换器
//! 支持 Kazumi 规则中常见的 XPath 表达式

use regex::Regex;
use std::sync::LazyLock;

/// 将 XPath 表达式转换为 CSS 选择器
/// 
/// 支持的 XPath 模式:
/// - `//div` → `div`
/// - `//div[1]` → `div:nth-of-type(1)`
/// - `//div[@class='x']` → `div.x`
/// - `//div[@id='x']` → `div#x`
/// - `//div[contains(@class, 'x')]` → `div[class*="x"]`
/// - `//div/a` → `div > a`
/// - `//div//a` → `div a`
/// - `//*[@id='x']` → `#x`
/// - `.//a` → `a` (相对路径)
pub fn xpath_to_css(xpath: &str) -> Result<CssSelector, String> {
    let xpath = xpath.trim();
    
    if xpath.is_empty() {
        return Err("空的 XPath 表达式".to_string());
    }

    // 解析并转换
    let (css, position_filter) = convert_xpath(xpath)?;
    
    Ok(CssSelector {
        selector: css,
        position_filter,
    })
}

/// CSS 选择器结果
#[derive(Debug, Clone)]
pub struct CssSelector {
    /// CSS 选择器字符串
    pub selector: String,
    /// 位置过滤器 (用于处理 position() > n 等)
    pub position_filter: Option<PositionFilter>,
}

/// 位置过滤器 (用于 position() > n 等无法用 CSS 表达的情况)
#[derive(Debug, Clone)]
pub enum PositionFilter {
    /// position() > n (跳过前 n 个元素)
    GreaterThan(usize),
}

// 正则表达式 (编译一次)
static RE_POSITION_INDEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[(\d+)\]").unwrap()
});

static RE_CLASS_ATTR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\[@class=['"]([^'"]+)['"]\]"#).unwrap()
});

static RE_ID_ATTR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\[@id=['"]([^'"]+)['"]\]"#).unwrap()
});

static RE_CONTAINS_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\[contains\s*\(\s*@class\s*,\s*['"]([^'"]+)['"]\s*\)\]"#).unwrap()
});

static RE_POSITION_GT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[position\s*\(\s*\)\s*>\s*(\d+)\]").unwrap()
});

static RE_GENERIC_ATTR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\[@([a-zA-Z_][a-zA-Z0-9_-]*)=['"]([^'"]+)['"]\]"#).unwrap()
});

fn convert_xpath(xpath: &str) -> Result<(String, Option<PositionFilter>), String> {
    let mut xpath = xpath.to_string();
    let mut position_filter = None;

    // 移除开头的 // 或 .// 或 /
    if xpath.starts_with(".//") {
        xpath = xpath[3..].to_string();
    } else if xpath.starts_with("//") {
        xpath = xpath[2..].to_string();
    } else if xpath.starts_with("./") {
        xpath = xpath[2..].to_string();
    } else if xpath.starts_with("/") {
        xpath = xpath[1..].to_string();
    }

    // 移除末尾的 /text()
    if xpath.ends_with("/text()") {
        xpath = xpath[..xpath.len() - 7].to_string();
    }

    // 检查 position() > n，需要在代码中过滤
    if let Some(caps) = RE_POSITION_GT.captures(&xpath) {
        if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
            position_filter = Some(PositionFilter::GreaterThan(n));
        }
        xpath = RE_POSITION_GT.replace_all(&xpath, "").to_string();
    }

    // 分割路径段
    let segments = split_xpath_segments(&xpath);
    let mut css = String::new();

    for (index, segment) in segments.iter().enumerate() {
        let css_segment = convert_segment(segment, index == 0)?;
        css.push_str(&css_segment);
    }

    // 清理多余空格
    let css = css.trim().to_string();
    
    Ok((css, position_filter))
}

/// 分割 XPath 路径段，处理 / 和 //
fn split_xpath_segments(xpath: &str) -> Vec<PathSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = xpath.chars().peekable();
    let mut is_descendant = false;

    while let Some(c) = chars.next() {
        if c == '/' {
            if !current.is_empty() {
                segments.push(PathSegment {
                    element: current.clone(),
                    is_descendant,
                });
                current.clear();
            }
            // 检查是否是 //
            is_descendant = chars.peek() == Some(&'/');
            if is_descendant {
                chars.next(); // 消耗第二个 /
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        segments.push(PathSegment {
            element: current,
            is_descendant,
        });
    }

    segments
}

#[derive(Debug)]
struct PathSegment {
    element: String,
    is_descendant: bool, // true = //, false = /
}

/// 转换单个路径段
fn convert_segment(segment: &PathSegment, is_first: bool) -> Result<String, String> {
    let mut element = segment.element.clone();
    
    // 第一个段不需要组合符，后续段根据是否为后代选择决定
    let combinator = if is_first {
        ""
    } else if segment.is_descendant {
        " " // 后代选择器用空格
    } else {
        " > " // 子选择器用 >
    };

    // 处理通配符 *
    if element == "*" || element.starts_with("*[") {
        element = element.replacen("*", "", 1);
    }

    // 处理 [@class='xxx']
    let element = RE_CLASS_ATTR.replace_all(&element, |caps: &regex::Captures| {
        let class_name = &caps[1];
        // 多个类名用空格分隔时，转换为 .class1.class2
        let classes: String = class_name
            .split_whitespace()
            .map(|c| format!(".{}", c))
            .collect();
        classes
    }).to_string();

    // 处理 [@id='xxx']
    let element = RE_ID_ATTR.replace_all(&element, |caps: &regex::Captures| {
        format!("#{}", &caps[1])
    }).to_string();

    // 处理 [contains(@class, 'xxx')]
    let element = RE_CONTAINS_CLASS.replace_all(&element, |caps: &regex::Captures| {
        format!("[class*=\"{}\"]", &caps[1])
    }).to_string();

    // 处理其他属性 [@attr='value']
    let element = RE_GENERIC_ATTR.replace_all(&element, |caps: &regex::Captures| {
        format!("[{}=\"{}\"]", &caps[1], &caps[2])
    }).to_string();

    // 处理位置索引 [n]
    let element = RE_POSITION_INDEX.replace_all(&element, |caps: &regex::Captures| {
        format!(":nth-of-type({})", &caps[1])
    }).to_string();

    // 组合结果
    Ok(format!("{}{}", combinator, element))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_xpath() {
        let result = xpath_to_css("//div").unwrap();
        assert_eq!(result.selector, "div");
    }

    #[test]
    fn test_nested_xpath() {
        let result = xpath_to_css("//div/a").unwrap();
        assert_eq!(result.selector, "div > a");
    }

    #[test]
    fn test_descendant_xpath() {
        let result = xpath_to_css("//div//a").unwrap();
        assert_eq!(result.selector, "div a");
    }

    #[test]
    fn test_position_index() {
        let result = xpath_to_css("//div[1]/a[2]").unwrap();
        assert_eq!(result.selector, "div:nth-of-type(1) > a:nth-of-type(2)");
    }

    #[test]
    fn test_class_attribute() {
        let result = xpath_to_css("//div[@class='item']").unwrap();
        assert_eq!(result.selector, "div.item");
    }

    #[test]
    fn test_id_attribute() {
        let result = xpath_to_css("//*[@id='main']").unwrap();
        assert_eq!(result.selector, "#main");
    }

    #[test]
    fn test_contains_class() {
        let result = xpath_to_css("//div[contains(@class, 'btn')]").unwrap();
        assert_eq!(result.selector, "div[class*=\"btn\"]");
    }

    #[test]
    fn test_complex_xpath() {
        let result = xpath_to_css("//div[1]/div[2]/div/ul/li").unwrap();
        assert_eq!(result.selector, "div:nth-of-type(1) > div:nth-of-type(2) > div > ul > li");
    }

    #[test]
    fn test_relative_xpath() {
        let result = xpath_to_css(".//a").unwrap();
        assert_eq!(result.selector, "a");
    }

    #[test]
    fn test_text_removal() {
        let result = xpath_to_css("//h3/a/text()").unwrap();
        assert_eq!(result.selector, "h3 > a");
    }

    #[test]
    fn test_position_filter() {
        let result = xpath_to_css("//div[position() > 1]").unwrap();
        assert_eq!(result.selector, "div");
        assert!(matches!(result.position_filter, Some(PositionFilter::GreaterThan(1))));
    }

    #[test]
    fn test_kazumi_rule_examples() {
        // AGE 规则
        let result = xpath_to_css("//div[2]/div/section/div/div/div/div").unwrap();
        assert!(result.selector.contains("div"));
        
        // class 选择
        let result = xpath_to_css("//div[@class='module-play-list']").unwrap();
        assert_eq!(result.selector, "div.module-play-list");
        
        // ul class
        let result = xpath_to_css("//ul[contains(@class, 'anthology-list-play')]").unwrap();
        assert_eq!(result.selector, "ul[class*=\"anthology-list-play\"]");
    }
}
