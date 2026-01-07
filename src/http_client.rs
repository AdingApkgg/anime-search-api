use once_cell::sync::Lazy;
use reqwest::{Client, Response};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

const TIMEOUT_SECONDS: u64 = 15;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 (AnimeSearch API)";

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

#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error("请求超时")]
    Timeout,
    #[error("请求失败: {0}")]
    RequestFailed(String),
    #[error("响应异常状态码: {0}")]
    BadStatus(u16),
}

/// GET 请求
pub async fn get(url: &str, referer: Option<&str>) -> Result<Response, HttpClientError> {
    let mut req = HTTP_CLIENT.get(url);
    
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

/// POST 请求 (Form body) 并返回文本
pub async fn post_form_text(
    url: &str,
    form: &HashMap<String, String>,
    referer: Option<&str>,
) -> Result<String, HttpClientError> {
    let mut req = HTTP_CLIENT.post(url).form(form);

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

    response
        .text()
        .await
        .map_err(|e| HttpClientError::RequestFailed(e.to_string()))
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
