use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::jupiter_rate_limiter::JupiterRateLimiter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BotType {
    MEV,
    Arbitrage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRegistration {
    pub bot_id: String,
    pub bot_type: BotType,
    pub priority: u8, // 1-10, higher = more priority
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub request_id: String,
    pub bot_id: String,
    pub opportunity: OpportunityData,
    pub estimated_profit_sol: f64,
    pub priority: u8,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityData {
    pub id: String,
    pub opportunity_type: String,
    pub tokens: Vec<String>,
    pub expected_profit_bps: u64,
    pub jupiter_route: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum CoordinatorMessage {
    RegisterBot(BotRegistration),
    SubmitExecution(ExecutionRequest),
    ExecutionComplete {
        request_id: String,
        success: bool,
        actual_profit_sol: Option<f64>,
    },
    GetStats,
}

pub struct BotCoordinator {
    registered_bots: Arc<RwLock<HashMap<String, BotRegistration>>>,
    execution_queue: Arc<RwLock<Vec<ExecutionRequest>>>,
    jupiter_limiter: Arc<JupiterRateLimiter>,
    message_tx: mpsc::UnboundedSender<CoordinatorMessage>,
    message_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<CoordinatorMessage>>>>,
    stats: Arc<RwLock<CoordinatorStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct CoordinatorStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_profit_sol: f64,
    pub executions_by_bot: HashMap<String, u64>,
    pub rate_limit_hits: u64,
}

impl BotCoordinator {
    pub fn new(jupiter_api_key: String) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            registered_bots: Arc::new(RwLock::new(HashMap::new())),
            execution_queue: Arc::new(RwLock::new(Vec::new())),
            jupiter_limiter: Arc::new(JupiterRateLimiter::new(jupiter_api_key)),
            message_tx: tx,
            message_rx: Arc::new(RwLock::new(Some(rx))),
            stats: Arc::new(RwLock::new(CoordinatorStats::default())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut rx = self
            .message_rx
            .write()
            .take()
            .ok_or_else(|| anyhow::anyhow!("Coordinator already started"))?;

        let registered_bots = self.registered_bots.clone();
        let execution_queue = self.execution_queue.clone();
        let jupiter_limiter = self.jupiter_limiter.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                match message {
                    CoordinatorMessage::RegisterBot(registration) => {
                        info!(
                            "Registering bot: {} ({:?})",
                            registration.bot_id, registration.bot_type
                        );
                        registered_bots
                            .write()
                            .insert(registration.bot_id.clone(), registration);
                    }

                    CoordinatorMessage::SubmitExecution(request) => {
                        debug!(
                            "Received execution request: {} from bot {}",
                            request.request_id, request.bot_id
                        );

                        // Add to queue and sort by priority and profit
                        {
                            let mut queue = execution_queue.write();
                            queue.push(request);
                            queue.sort_by(|a, b| {
                                b.priority.cmp(&a.priority).then(
                                    b.estimated_profit_sol
                                        .partial_cmp(&a.estimated_profit_sol)
                                        .unwrap_or(std::cmp::Ordering::Equal),
                                )
                            });
                        }

                        // Process the queue
                        Self::process_execution_queue(
                            execution_queue.clone(),
                            jupiter_limiter.clone(),
                            stats.clone(),
                        )
                        .await;
                    }

                    CoordinatorMessage::ExecutionComplete {
                        request_id,
                        success,
                        actual_profit_sol,
                    } => {
                        debug!("Execution {} completed: success={}", request_id, success);

                        let mut stats_guard = stats.write();
                        stats_guard.total_executions += 1;

                        if success {
                            stats_guard.successful_executions += 1;
                            if let Some(profit) = actual_profit_sol {
                                stats_guard.total_profit_sol += profit;
                            }
                        } else {
                            stats_guard.failed_executions += 1;
                        }
                    }

                    CoordinatorMessage::GetStats => {
                        let stats_guard = stats.read();
                        info!("Coordinator Stats: {:?}", *stats_guard);
                    }
                }
            }
        });

        Ok(())
    }

    async fn process_execution_queue(
        execution_queue: Arc<RwLock<Vec<ExecutionRequest>>>,
        jupiter_limiter: Arc<JupiterRateLimiter>,
        stats: Arc<RwLock<CoordinatorStats>>,
    ) {
        let request = {
            let mut queue = execution_queue.write();
            queue.pop()
        };

        if let Some(request) = request {
            // Execute the trade using Jupiter
            match Self::execute_jupiter_trade(&jupiter_limiter, &request).await {
                Ok(actual_profit) => {
                    info!(
                        "Successfully executed trade {} with profit: {:.4} SOL",
                        request.request_id, actual_profit
                    );

                    let mut stats_guard = stats.write();
                    stats_guard.total_executions += 1;
                    stats_guard.successful_executions += 1;
                    stats_guard.total_profit_sol += actual_profit;
                    *stats_guard
                        .executions_by_bot
                        .entry(request.bot_id)
                        .or_insert(0) += 1;
                }
                Err(e) => {
                    warn!("Failed to execute trade {}: {}", request.request_id, e);

                    let mut stats_guard = stats.write();
                    stats_guard.total_executions += 1;
                    stats_guard.failed_executions += 1;
                    *stats_guard
                        .executions_by_bot
                        .entry(request.bot_id)
                        .or_insert(0) += 1;

                    if e.to_string().contains("rate limit") {
                        stats_guard.rate_limit_hits += 1;
                    }
                }
            }
        }
    }

    async fn execute_jupiter_trade(
        jupiter_limiter: &JupiterRateLimiter,
        request: &ExecutionRequest,
    ) -> Result<f64> {
        // Execute the Jupiter swap
        let _swap_response: serde_json::Value = jupiter_limiter
            .execute_request("/v6/swap", Some(request.opportunity.jupiter_route.clone()))
            .await?;

        // Parse the response and calculate actual profit
        // This is simplified - you'd need to parse the actual transaction result
        let actual_profit = request.estimated_profit_sol * 0.95; // Assume 5% slippage for now

        Ok(actual_profit)
    }

    pub fn get_handle(&self) -> CoordinatorHandle {
        CoordinatorHandle {
            message_tx: self.message_tx.clone(),
        }
    }

    pub async fn get_stats(&self) -> CoordinatorStats {
        self.stats.read().clone()
    }
}

#[derive(Clone)]
pub struct CoordinatorHandle {
    message_tx: mpsc::UnboundedSender<CoordinatorMessage>,
}

impl CoordinatorHandle {
    pub fn register_bot(&self, registration: BotRegistration) -> Result<()> {
        self.message_tx
            .send(CoordinatorMessage::RegisterBot(registration))?;
        Ok(())
    }

    pub fn submit_execution(&self, request: ExecutionRequest) -> Result<()> {
        self.message_tx
            .send(CoordinatorMessage::SubmitExecution(request))?;
        Ok(())
    }

    pub fn execution_complete(
        &self,
        request_id: String,
        success: bool,
        actual_profit_sol: Option<f64>,
    ) -> Result<()> {
        self.message_tx
            .send(CoordinatorMessage::ExecutionComplete {
                request_id,
                success,
                actual_profit_sol,
            })?;
        Ok(())
    }

    pub fn get_stats(&self) -> Result<()> {
        self.message_tx.send(CoordinatorMessage::GetStats)?;
        Ok(())
    }
}
