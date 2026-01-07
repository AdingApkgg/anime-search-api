//! 核心搜索逻辑
//! 处理并发搜索和 SSE 流式响应

use crate::engine::{search_with_rule, search_with_rule_and_episodes};
use crate::types::{Rule, StreamEvent, StreamProgress, StreamResult};
use futures::stream::Stream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info};

/// 使用指定规则执行流式搜索 (不获取集数)
#[allow(dead_code)]
pub fn search_stream_with_rules(
    keyword: String,
    rules: Vec<Arc<Rule>>,
) -> impl Stream<Item = String> {
    search_stream_with_rules_options(keyword, rules, false)
}

/// 使用指定规则执行流式搜索 (可选获取集数)
pub fn search_stream_with_rules_options(
    keyword: String,
    rules: Vec<Arc<Rule>>,
    fetch_episodes: bool,
) -> impl Stream<Item = String> {
    let (tx, rx) = mpsc::channel::<String>(100);

    tokio::spawn(async move {
        execute_parallel_search(keyword, rules, tx, fetch_episodes).await;
    });

    ReceiverStream::new(rx)
}

/// 并行执行搜索
async fn execute_parallel_search(
    keyword: String,
    rules: Vec<Arc<Rule>>,
    tx: mpsc::Sender<String>,
    fetch_episodes: bool,
) {
    let total = rules.len();
    let completed = Arc::new(AtomicUsize::new(0));

    info!("开始搜索: {}, 共 {} 个规则, 获取集数: {}", keyword, total, fetch_episodes);

    // 发送初始事件
    let init_event = StreamEvent::Init { total };
    if tx.send(format_event(&init_event)).await.is_err() {
        return;
    }

    // 并行搜索所有平台
    let mut handles = Vec::new();

    for rule in rules {
        let keyword = keyword.clone();
        let tx = tx.clone();
        let completed = completed.clone();

        let handle = tokio::spawn(async move {
            let result = if fetch_episodes {
                search_with_rule_and_episodes(&rule, &keyword).await
            } else {
                search_with_rule(&rule, &keyword).await
            };
            let current = completed.fetch_add(1, Ordering::SeqCst) + 1;

            let progress = StreamProgress {
                completed: current,
                total,
            };

            debug!("规则 {} 搜索完成: {} 个结果", rule.name, result.count);

            // 只有有结果或有错误时才发送结果
            let event = if result.count > 0 || result.error.is_some() {
                let stream_result = StreamResult {
                    name: rule.name.clone(),
                    color: if result.error.is_some() {
                        "red".to_string()
                    } else {
                        rule.color.clone()
                    },
                    tags: rule.tags.clone(),
                    items: result.items,
                    error: result.error,
                };
                StreamEvent::Result {
                    progress,
                    result: stream_result,
                }
            } else {
                StreamEvent::Progress { progress }
            };

            let _ = tx.send(format_event(&event)).await;
        });

        handles.push(handle);
    }

    // 等待所有搜索完成
    for handle in handles {
        let _ = handle.await;
    }

    // 发送完成信号
    let done_event = StreamEvent::Done { done: true };
    let _ = tx.send(format_event(&done_event)).await;

    info!("搜索完成: {}", keyword);
}

/// 格式化 SSE 事件
fn format_event(event: &StreamEvent) -> String {
    format!("{}\n", serde_json::to_string(event).unwrap_or_default())
}
