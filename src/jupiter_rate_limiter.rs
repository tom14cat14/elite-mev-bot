use anyhow::Result;
use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use parking_lot::Mutex;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, Instant};
use tracing::{debug, error, warn};

pub struct JupiterRateLimiter {
    limiter: Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
        >,
    >,
    client: Client,
    api_key: String,
    burst_protection: Arc<Mutex<BurstProtector>>,
}

struct BurstProtector {
    last_request: Option<Instant>,
    consecutive_requests: u32,
    min_interval_ms: u64,
}

impl JupiterRateLimiter {
    pub fn new(api_key: String) -> Self {
        // Jupiter Ultra: 50 requests per 10 seconds rolling window
        // Conservative: 4 requests per second with burst of 8 (safety margin)
        let quota = Quota::per_second(nonzero!(4u32)).allow_burst(nonzero!(8u32));
        let limiter = Arc::new(RateLimiter::direct(quota));

        let client = Client::builder()
            .timeout(Duration::from_secs(15)) // Faster timeout for arbitrage
            .build()
            .expect("Failed to create HTTP client");

        let burst_protection = Arc::new(Mutex::new(BurstProtector {
            last_request: None,
            consecutive_requests: 0,
            min_interval_ms: 250, // Minimum 250ms between requests for safety
        }));

        Self {
            limiter,
            client,
            api_key,
            burst_protection,
        }
    }

    /// Execute a Jupiter API request with rate limiting and exponential backoff
    pub async fn execute_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        request_body: Option<Value>,
    ) -> Result<T> {
        self.wait_for_rate_limit().await?;
        self.apply_burst_protection().await;

        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 5;

        loop {
            match self.make_request(endpoint, request_body.clone()).await {
                Ok(response) => {
                    // Reset burst protection on success
                    {
                        let mut burst = self.burst_protection.lock();
                        burst.consecutive_requests = 0;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count >= MAX_RETRIES {
                        error!(
                            "Jupiter API request failed after {} retries: {}",
                            MAX_RETRIES, e
                        );
                        return Err(e);
                    }

                    // Check if it's a rate limit error
                    let is_rate_limit_error = e.to_string().contains("429")
                        || e.to_string().contains("rate limit")
                        || e.to_string().contains("Too Many Requests");

                    if is_rate_limit_error {
                        // Exponential backoff for rate limit errors
                        let backoff_ms = self.calculate_exponential_backoff(retry_count);
                        warn!(
                            "Rate limit hit, backing off for {}ms (attempt {}/{})",
                            backoff_ms, retry_count, MAX_RETRIES
                        );
                        sleep(Duration::from_millis(backoff_ms)).await;

                        // Update burst protection
                        {
                            let mut burst = self.burst_protection.lock();
                            burst.consecutive_requests += 1;
                            burst.min_interval_ms = std::cmp::min(burst.min_interval_ms * 2, 2000);
                        }
                    } else {
                        // Regular exponential backoff for other errors
                        let backoff_ms = std::cmp::min(100 * (2_u64.pow(retry_count)), 5000);
                        warn!(
                            "Jupiter API error, retrying in {}ms (attempt {}/{}): {}",
                            backoff_ms, retry_count, MAX_RETRIES, e
                        );
                        sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }
    }

    async fn wait_for_rate_limit(&self) -> Result<()> {
        self.limiter.until_ready().await;
        debug!("Rate limiter cleared, proceeding with request");
        Ok(())
    }

    async fn apply_burst_protection(&self) {
        let (wait_time, min_interval_ms) = {
            let burst = self.burst_protection.lock();

            let wait_time = if let Some(last_request) = burst.last_request {
                let elapsed = last_request.elapsed();
                let min_interval = Duration::from_millis(burst.min_interval_ms);

                if elapsed < min_interval {
                    Some(min_interval - elapsed)
                } else {
                    None
                }
            } else {
                None
            };

            (wait_time, burst.min_interval_ms)
        };

        if let Some(wait_time) = wait_time {
            debug!("Burst protection: waiting {}ms", wait_time.as_millis());
            sleep(wait_time).await;
        }

        {
            let mut burst = self.burst_protection.lock();
            burst.last_request = Some(Instant::now());
        }
    }

    async fn make_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        request_body: Option<Value>,
    ) -> Result<T> {
        let url = format!("https://api.jup.ag/ultra{}", endpoint);

        let mut request_builder = if let Some(body) = request_body {
            self.client.post(&url).json(&body)
        } else {
            self.client.get(&url)
        };

        request_builder = request_builder.header("x-api-key", &self.api_key);

        let response = request_builder.send().await?;

        if response.status().is_success() {
            let result = response.json::<T>().await?;
            debug!("Jupiter API request successful to {}", endpoint);
            Ok(result)
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "Jupiter API error {}: {}",
                status,
                error_body
            ))
        }
    }

    fn calculate_exponential_backoff(&self, retry_count: u32) -> u64 {
        // Aggressive exponential backoff for Jupiter Ultra rolling window
        // 1s, 2s, 4s, 8s, 16s (respects 10-second rolling window)
        let base_delay = 1000 * (2_u64.pow(retry_count.saturating_sub(1)));
        let max_delay = 16000; // Max 16s for rolling window recovery
        let jitter = fastrand::u64(0..200); // Add jitter for distributed requests

        std::cmp::min(base_delay + jitter, max_delay)
    }

    /// Get current rate limiter statistics
    pub fn get_stats(&self) -> RateLimiterStats {
        let burst = self.burst_protection.lock();
        RateLimiterStats {
            consecutive_requests: burst.consecutive_requests,
            min_interval_ms: burst.min_interval_ms,
            last_request: burst.last_request.map(|_t| Utc::now()),
        }
    }
}

#[derive(Debug)]
pub struct RateLimiterStats {
    pub consecutive_requests: u32,
    pub min_interval_ms: u64,
    pub last_request: Option<DateTime<Utc>>,
}

// Add fastrand as a dependency in Cargo.toml
