use once_cell::sync::Lazy;
use reqwest::{Client, Response};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

const TIMEOUT_SECONDS: u64 = 15;
const RETRY_TIMEOUT_SECONDS: u64 = 20;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 (AnimeSearch API)";

/// 反代前缀 (用于网络问题时重试)
const PROXY_PREFIX: &str = "https://rp.30hb.cn/?target=";

/// 全局 HTTP 客户端
pub static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .user_agent(USER_AGENT)
        .gzip(true)
        .brotli(true)
        .danger_accept_invalid_certs(true) // 某些站点证书有问题
        .build()
        .expect("Failed to create HTTP client")
});

/// 用于重试的 HTTP 客户端 (更长超时)
pub static RETRY_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(RETRY_TIMEOUT_SECONDS))
        .user_agent(USER_AGENT)
        .gzip(true)
        .brotli(true)
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to create retry HTTP client")
});

#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error("请求超时")]
    Timeout,
    #[error("请求失败: {0}")]
    RequestFailed(String),
    #[error("响应异常状态码: {0}")]
    BadStatus(u16),
}

/// 判断是否应该使用反代重试
fn should_retry(error: &HttpClientError) -> bool {
    matches!(
        error,
        HttpClientError::Timeout
            | HttpClientError::RequestFailed(_)
    )
}

/// 判断状态码是否应该重试
fn should_retry_status(status: u16) -> bool {
    // 403, 404, 500+ 等可能是反爬，尝试反代
    matches!(status, 403 | 429 | 500..=599)
}

/// GET 请求 (内部实现)
async fn get_internal(client: &Client, url: &str, referer: Option<&str>) -> Result<Response, HttpClientError> {
    let mut req = client.get(url);
    
    if let Some(ref_url) = referer {
        req = req.header("Referer", ref_url);
    }
    
    req = req
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Connection", "keep-alive");

    let response = req.send().await.map_err(|e| {
        if e.is_timeout() {
            HttpClientError::Timeout
        } else {
            HttpClientError::RequestFailed(e.to_string())
        }
    })?;

    if !response.status().is_success() {
        return Err(HttpClientError::BadStatus(response.status().as_u16()));
    }

    Ok(response)
}

/// GET 请求 (自动重试反代)
pub async fn get(url: &str, referer: Option<&str>) -> Result<Response, HttpClientError> {
    // 第一次尝试直连
    match get_internal(&HTTP_CLIENT, url, referer).await {
        Ok(resp) => Ok(resp),
        Err(e) => {
            // 网络问题或反爬状态码，尝试反代
            let should_use_proxy = match &e {
                HttpClientError::BadStatus(status) => should_retry_status(*status),
                _ => should_retry(&e),
            };

            if should_use_proxy {
                let proxy_url = format!("{}{}", PROXY_PREFIX, url);
                tracing::debug!("使用反代重试: {}", url);
                get_internal(&RETRY_CLIENT, &proxy_url, referer).await
            } else {
                Err(e)
            }
        }
    }
}

/// GET 请求并返回文本
pub async fn get_text(url: &str, referer: Option<&str>) -> Result<String, HttpClientError> {
    let response = get(url, referer).await?;
    response
        .text()
        .await
        .map_err(|e| HttpClientError::RequestFailed(e.to_string()))
}

/// GET 请求并返回 JSON
#[allow(dead_code)]
pub async fn get_json<T: serde::de::DeserializeOwned>(
    url: &str,
    referer: Option<&str>,
) -> Result<T, HttpClientError> {
    let response = get(url, referer).await?;
    response
        .json()
        .await
        .map_err(|e| HttpClientError::RequestFailed(e.to_string()))
}

/// POST 请求 (Form body) 内部实现
async fn post_form_internal(
    client: &Client,
    url: &str,
    form: &HashMap<String, String>,
    referer: Option<&str>,
) -> Result<Response, HttpClientError> {
    let mut req = client.post(url).form(form);

    if let Some(ref_url) = referer {
        req = req.header("Referer", ref_url);
    }

    req = req
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Connection", "keep-alive");

    let response = req.send().await.map_err(|e| {
        if e.is_timeout() {
            HttpClientError::Timeout
        } else {
            HttpClientError::RequestFailed(e.to_string())
        }
    })?;

    if !response.status().is_success() {
        return Err(HttpClientError::BadStatus(response.status().as_u16()));
    }

    Ok(response)
}

/// POST 请求 (Form body) 并返回文本 (自动重试反代)
pub async fn post_form_text(
    url: &str,
    form: &HashMap<String, String>,
    referer: Option<&str>,
) -> Result<String, HttpClientError> {
    // 第一次尝试直连
    match post_form_internal(&HTTP_CLIENT, url, form, referer).await {
        Ok(resp) => resp
            .text()
            .await
            .map_err(|e| HttpClientError::RequestFailed(e.to_string())),
        Err(e) => {
            // 网络问题或反爬状态码，尝试反代
            let should_use_proxy = match &e {
                HttpClientError::BadStatus(status) => should_retry_status(*status),
                _ => should_retry(&e),
            };

            if should_use_proxy {
                let proxy_url = format!("{}{}", PROXY_PREFIX, url);
                tracing::debug!("使用反代重试 POST: {}", url);
                let resp = post_form_internal(&RETRY_CLIENT, &proxy_url, form, referer).await?;
                resp.text()
                    .await
                    .map_err(|e| HttpClientError::RequestFailed(e.to_string()))
            } else {
                Err(e)
            }
        }
    }
}

/// POST 请求 (JSON body)
#[allow(dead_code)]
pub async fn post_json<T: serde::Serialize>(
    url: &str,
    body: &T,
    referer: Option<&str>,
) -> Result<Response, HttpClientError> {
    let mut req = HTTP_CLIENT.post(url).json(body);

    if let Some(ref_url) = referer {
        req = req.header("Referer", ref_url);
    }

    let response = req.send().await.map_err(|e| {
        if e.is_timeout() {
            HttpClientError::Timeout
        } else {
            HttpClientError::RequestFailed(e.to_string())
        }
    })?;

    if !response.status().is_success() {
        return Err(HttpClientError::BadStatus(response.status().as_u16()));
    }

    Ok(response)
}
