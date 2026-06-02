use std::time::Duration;

use axum::http::StatusCode;
use reqwest::{Response, RequestBuilder};

use datum::config::RetryConfig;

/// Send an HTTP request with retry logic.
///
/// `build` is called once per attempt to construct a fresh `reqwest::RequestBuilder`.
/// Retries only on network errors or status codes listed in `retry_on_status`.
/// Uses exponential backoff: `min(base_delay * 2^attempt, max_delay)`.
pub async fn dispatch<F>(
    retry: Option<&RetryConfig>,
    build: F,
) -> Result<Response, StatusCode>
where
    F: Fn() -> RequestBuilder,
{
    if let Some(cfg) = retry {
        for attempt in 0..=cfg.retry_maxnum {
            if attempt > 0 {
                let delay_ms = (cfg.base_delay_ms * 2u64.pow(attempt.saturating_sub(1))).min(cfg.most_delay_ms);
                eprintln!("🔁 Retry attempt {attempt}/{max} after {delay_ms}ms", max = cfg.retry_maxnum);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            match build().send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    }
                    let code = response.status().as_u16();
                    if !cfg.retry_status.contains(&code) {
                        return Err(StatusCode::BAD_GATEWAY);
                    }
                    eprintln!("⚠️ Upstream returned {code}, attempt {}/{}", attempt + 1, cfg.retry_maxnum + 1);
                    // 消耗响应体来释放连接
                    let _ = response.bytes().await;
                }
                Err(e) => {
                    eprintln!("❌ Upstream request failed (attempt {}/{}): {e}", attempt + 1, cfg.retry_maxnum + 1);
                }
            }
        }

        eprintln!("❌ All {} retry attempts exhausted", cfg.retry_maxnum + 1);
        Err(StatusCode::BAD_GATEWAY)
    } else {
        build().send().await
            .map_err(|e| {
                eprintln!("❌ Upstream request failed: {e}");
                StatusCode::BAD_GATEWAY
            })
    }
}
