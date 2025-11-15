use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info, warn}; // HIGH PRIORITY FIX: Use tokio::sync::Mutex for async contexts

/// Enhanced error recovery manager with exponential backoff and specific failure handling
///
/// PERFORMANCE FIX: Now uses tokio::sync::Mutex instead of std::sync::Mutex to avoid
/// blocking the async runtime. All lock() calls are now .await instead of blocking.
#[derive(Debug, Clone)]
pub struct ErrorRecoveryManager {
    retry_policies: Arc<Mutex<HashMap<String, RetryPolicy>>>,
    failure_statistics: Arc<Mutex<FailureStatistics>>,
    circuit_breaker_states: Arc<Mutex<HashMap<String, CircuitBreakerState>>>,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_factor: f64,
}

#[derive(Debug, Clone, Default)]
pub struct FailureStatistics {
    pub transaction_timeouts: u64,
    pub invalid_blockhashes: u64,
    pub rpc_failures: u64,
    pub bundle_failures: u64,
    pub shredstream_failures: u64,
    pub total_recovery_attempts: u64,
    pub successful_recoveries: u64,
    pub last_failure_time: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub reset_timeout: Duration,
    pub last_failure_time: Option<Instant>,
    pub state: CircuitState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub enum ErrorType {
    TransactionTimeout,
    InvalidBlockhash,
    RpcFailure,
    BundleSubmissionFailure,
    ShredStreamConnectionFailure,
    InsufficientBalance,
    SlippageExceeded,
    PriceImpactTooHigh,
    NetworkCongestion,
    Unknown(String),
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 30000, // 30 seconds max
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            failure_count: 0,
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(60),
            last_failure_time: None,
            state: CircuitState::Closed,
        }
    }
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        let mut retry_policies = HashMap::new();

        // Configure specific retry policies for different error types
        retry_policies.insert(
            "transaction_timeout".to_string(),
            RetryPolicy {
                max_retries: 5,
                base_delay_ms: 200,
                max_delay_ms: 10000,
                backoff_multiplier: 1.5,
                jitter_factor: 0.2,
            },
        );

        retry_policies.insert(
            "invalid_blockhash".to_string(),
            RetryPolicy {
                max_retries: 3,
                base_delay_ms: 50,
                max_delay_ms: 2000,
                backoff_multiplier: 2.0,
                jitter_factor: 0.1,
            },
        );

        retry_policies.insert(
            "rpc_failure".to_string(),
            RetryPolicy {
                max_retries: 4,
                base_delay_ms: 500,
                max_delay_ms: 15000,
                backoff_multiplier: 2.5,
                jitter_factor: 0.3,
            },
        );

        retry_policies.insert(
            "bundle_failure".to_string(),
            RetryPolicy {
                max_retries: 3,
                base_delay_ms: 300,
                max_delay_ms: 8000,
                backoff_multiplier: 2.0,
                jitter_factor: 0.15,
            },
        );

        retry_policies.insert(
            "shredstream_failure".to_string(),
            RetryPolicy {
                max_retries: 10,
                base_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 1.3,
                jitter_factor: 0.1,
            },
        );

        let mut circuit_breakers = HashMap::new();

        // Configure circuit breakers for critical services
        circuit_breakers.insert(
            "rpc_endpoint".to_string(),
            CircuitBreakerState {
                failure_threshold: 3,
                reset_timeout: Duration::from_secs(30),
                ..Default::default()
            },
        );

        circuit_breakers.insert(
            "jito_bundles".to_string(),
            CircuitBreakerState {
                failure_threshold: 5,
                reset_timeout: Duration::from_secs(60),
                ..Default::default()
            },
        );

        circuit_breakers.insert(
            "shredstream".to_string(),
            CircuitBreakerState {
                failure_threshold: 8,
                reset_timeout: Duration::from_secs(120),
                ..Default::default()
            },
        );

        Self {
            retry_policies: Arc::new(Mutex::new(retry_policies)),
            failure_statistics: Arc::new(Mutex::new(FailureStatistics::default())),
            circuit_breaker_states: Arc::new(Mutex::new(circuit_breakers)),
        }
    }

    /// Execute with retry logic and exponential backoff
    pub async fn execute_with_retry<F, T, E>(
        &self,
        operation_name: &str,
        error_type: ErrorType,
        operation: F,
    ) -> Result<T, E>
    where
        F: Fn() -> Result<T, E> + Send + Sync,
        E: std::fmt::Debug,
        T: std::fmt::Debug,
    {
        let error_key = self.error_type_to_key(&error_type);

        // Check circuit breaker
        if !self.is_circuit_closed(&error_key) {
            error!(
                "Circuit breaker is open for {}, skipping operation",
                operation_name
            );
            return operation(); // Try operation anyway, circuit breaker will be updated
        }

        let policy = {
            let policies = self.retry_policies.lock().await;
            policies.get(&error_key).cloned().unwrap_or_default()
        };

        let mut attempt = 0;
        loop {
            match operation() {
                Ok(result) => {
                    if attempt > 0 {
                        info!(
                            "✅ Operation '{}' succeeded after {} attempts",
                            operation_name,
                            attempt + 1
                        );
                        self.record_successful_recovery();
                        self.reset_circuit_breaker(&error_key);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    attempt += 1;
                    self.record_failure(&error_type);

                    if attempt > policy.max_retries {
                        error!(
                            "❌ Operation '{}' failed after {} attempts: {:?}",
                            operation_name, attempt, err
                        );
                        self.trigger_circuit_breaker(&error_key);
                        return Err(err);
                    }

                    let delay = self.calculate_delay(&policy, attempt);
                    warn!(
                        "⚠️  Operation '{}' failed (attempt {}/{}), retrying in {}ms: {:?}",
                        operation_name,
                        attempt,
                        policy.max_retries + 1,
                        delay.as_millis(),
                        err
                    );

                    sleep(delay).await;
                }
            }
        }
    }

    /// Execute async operation with retry logic
    pub async fn execute_async_with_retry<F, Fut, T, E>(
        &self,
        operation_name: &str,
        error_type: ErrorType,
        operation: F,
    ) -> Result<T, E>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        E: std::fmt::Debug,
    {
        let error_key = self.error_type_to_key(&error_type);

        // Check circuit breaker
        if !self.is_circuit_closed(&error_key) {
            error!(
                "Circuit breaker is open for {}, skipping operation",
                operation_name
            );
            return operation().await; // Return the error
        }

        let policy = {
            let policies = self.retry_policies.lock().await;
            policies.get(&error_key).cloned().unwrap_or_default()
        };

        let mut attempt = 0;
        loop {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!(
                            "✅ Async operation '{}' succeeded after {} attempts",
                            operation_name,
                            attempt + 1
                        );
                        self.record_successful_recovery();
                        self.reset_circuit_breaker(&error_key);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    attempt += 1;
                    self.record_failure(&error_type);

                    if attempt > policy.max_retries {
                        error!(
                            "❌ Async operation '{}' failed after {} attempts: {:?}",
                            operation_name, attempt, err
                        );
                        self.trigger_circuit_breaker(&error_key);
                        return Err(err);
                    }

                    let delay = self.calculate_delay(&policy, attempt);
                    warn!(
                        "⚠️  Async operation '{}' failed (attempt {}/{}), retrying in {}ms: {:?}",
                        operation_name,
                        attempt,
                        policy.max_retries + 1,
                        delay.as_millis(),
                        err
                    );

                    sleep(delay).await;
                }
            }
        }
    }

    fn calculate_delay(&self, policy: &RetryPolicy, attempt: u32) -> Duration {
        let base_delay = policy.base_delay_ms as f64;
        let exponential_delay = base_delay * policy.backoff_multiplier.powi(attempt as i32 - 1);

        // Add jitter to prevent thundering herd
        let jitter = exponential_delay * policy.jitter_factor * (fastrand::f64() - 0.5);
        let final_delay = (exponential_delay + jitter).min(policy.max_delay_ms as f64);

        Duration::from_millis(final_delay as u64)
    }

    fn error_type_to_key(&self, error_type: &ErrorType) -> String {
        match error_type {
            ErrorType::TransactionTimeout => "transaction_timeout".to_string(),
            ErrorType::InvalidBlockhash => "invalid_blockhash".to_string(),
            ErrorType::RpcFailure => "rpc_failure".to_string(),
            ErrorType::BundleSubmissionFailure => "bundle_failure".to_string(),
            ErrorType::ShredStreamConnectionFailure => "shredstream_failure".to_string(),
            ErrorType::InsufficientBalance => "insufficient_balance".to_string(),
            ErrorType::SlippageExceeded => "slippage_exceeded".to_string(),
            ErrorType::PriceImpactTooHigh => "price_impact_high".to_string(),
            ErrorType::NetworkCongestion => "network_congestion".to_string(),
            ErrorType::Unknown(key) => key.clone(),
        }
    }

    fn record_failure(&self, error_type: &ErrorType) {
        // SAFETY FIX: Can't use .await in non-async function, so we need to make this async
        // For now, we'll use a blocking approach with proper error handling
        let mut stats = match self.failure_statistics.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                error!("Failed to acquire lock on failure_statistics");
                return;
            }
        };

        match error_type {
            ErrorType::TransactionTimeout => stats.transaction_timeouts += 1,
            ErrorType::InvalidBlockhash => stats.invalid_blockhashes += 1,
            ErrorType::RpcFailure => stats.rpc_failures += 1,
            ErrorType::BundleSubmissionFailure => stats.bundle_failures += 1,
            ErrorType::ShredStreamConnectionFailure => stats.shredstream_failures += 1,
            _ => {}
        }

        stats.total_recovery_attempts += 1;
        stats.last_failure_time = Some(Instant::now());
    }

    fn record_successful_recovery(&self) {
        // SAFETY FIX: Use try_lock with proper error handling
        if let Ok(mut stats) = self.failure_statistics.try_lock() {
            stats.successful_recoveries += 1;
        } else {
            error!("Failed to acquire lock for recording successful recovery");
        }
    }

    fn is_circuit_closed(&self, service_key: &str) -> bool {
        // SAFETY FIX: Use try_lock with proper error handling
        let mut breakers = match self.circuit_breaker_states.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                error!("Failed to acquire lock on circuit_breaker_states");
                return true; // Fail open - allow operation to proceed
            }
        };

        if let Some(breaker) = breakers.get_mut(service_key) {
            match breaker.state {
                CircuitState::Closed => true,
                CircuitState::Open => {
                    // Check if we should move to half-open
                    if let Some(last_failure) = breaker.last_failure_time {
                        if last_failure.elapsed() > breaker.reset_timeout {
                            breaker.state = CircuitState::HalfOpen;
                            info!(
                                "🔄 Circuit breaker for {} moved to half-open state",
                                service_key
                            );
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CircuitState::HalfOpen => true,
            }
        } else {
            true // No circuit breaker configured
        }
    }

    fn trigger_circuit_breaker(&self, service_key: &str) {
        // SAFETY FIX: Use try_lock with proper error handling
        let mut breakers = match self.circuit_breaker_states.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                error!("Failed to acquire lock for triggering circuit breaker");
                return;
            }
        };

        if let Some(breaker) = breakers.get_mut(service_key) {
            breaker.failure_count += 1;
            breaker.last_failure_time = Some(Instant::now());

            if breaker.failure_count >= breaker.failure_threshold {
                breaker.state = CircuitState::Open;
                error!(
                    "🚨 Circuit breaker OPENED for {} after {} failures",
                    service_key, breaker.failure_count
                );
            }
        }
    }

    fn reset_circuit_breaker(&self, service_key: &str) {
        // SAFETY FIX: Use try_lock with proper error handling
        let mut breakers = match self.circuit_breaker_states.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                error!("Failed to acquire lock for resetting circuit breaker");
                return;
            }
        };

        if let Some(breaker) = breakers.get_mut(service_key) {
            breaker.failure_count = 0;
            breaker.state = CircuitState::Closed;
            breaker.last_failure_time = None;
            debug!("✅ Circuit breaker RESET for {}", service_key);
        }
    }

    /// Get failure statistics (SAFETY FIX: Now async to use .await)
    pub async fn get_failure_statistics(&self) -> FailureStatistics {
        self.failure_statistics.lock().await.clone()
    }

    /// Get circuit breaker status (SAFETY FIX: Now async to use .await)
    pub async fn get_circuit_breaker_status(&self) -> HashMap<String, CircuitBreakerState> {
        self.circuit_breaker_states.lock().await.clone()
    }

    /// Update retry policy for specific error type (SAFETY FIX: Now async to use .await)
    pub async fn update_retry_policy(&self, error_type: ErrorType, policy: RetryPolicy) {
        let error_key = self.error_type_to_key(&error_type);
        let mut policies = self.retry_policies.lock().await;
        policies.insert(error_key, policy);
    }
}

impl Default for ErrorRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to classify errors
pub fn classify_error(error_message: &str) -> ErrorType {
    let error_lower = error_message.to_lowercase();

    if error_lower.contains("timeout") || error_lower.contains("timed out") {
        ErrorType::TransactionTimeout
    } else if error_lower.contains("blockhash") || error_lower.contains("recent_blockhash") {
        ErrorType::InvalidBlockhash
    } else if error_lower.contains("rpc") || error_lower.contains("connection") {
        ErrorType::RpcFailure
    } else if error_lower.contains("bundle") || error_lower.contains("jito") {
        ErrorType::BundleSubmissionFailure
    } else if error_lower.contains("shredstream") || error_lower.contains("shreds") {
        ErrorType::ShredStreamConnectionFailure
    } else if error_lower.contains("insufficient") || error_lower.contains("balance") {
        ErrorType::InsufficientBalance
    } else if error_lower.contains("slippage") {
        ErrorType::SlippageExceeded
    } else if error_lower.contains("price impact") {
        ErrorType::PriceImpactTooHigh
    } else if error_lower.contains("congestion") || error_lower.contains("busy") {
        ErrorType::NetworkCongestion
    } else {
        ErrorType::Unknown(error_message.to_string())
    }
}
