use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Comprehensive monitoring and alerting system for MEV bot
#[derive(Debug)]
pub struct MonitoringSystem {
    prometheus_client: Option<PrometheusClient>,
    grafana_client: Option<GrafanaClient>,
    pagerduty_client: Option<PagerDutyClient>,
    slack_webhook: Option<String>,
    discord_webhook: Option<String>,
    metrics: Arc<Mutex<SystemMetrics>>,
    alert_rules: Arc<Mutex<Vec<AlertRule>>>,
    active_alerts: Arc<Mutex<HashMap<String, Alert>>>,
    http_client: Client,
}

#[derive(Debug, Clone)]
pub struct PrometheusClient {
    pub endpoint: String,
    pub job_name: String,
    pub instance: String,
}

#[derive(Debug, Clone)]
pub struct GrafanaClient {
    pub endpoint: String,
    pub api_key: String,
    pub dashboard_uid: String,
}

#[derive(Debug, Clone)]
pub struct PagerDutyClient {
    pub integration_key: String,
    pub service_key: String,
    pub api_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    // Trading metrics
    pub total_trades: u64,
    pub successful_trades: u64,
    pub failed_trades: u64,
    pub total_profit_sol: f64,
    pub total_loss_sol: f64,
    pub win_rate: f64,

    // Performance metrics
    pub average_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub throughput_ops_per_sec: f64,

    // System metrics
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub disk_usage_percent: f64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,

    // Bot-specific metrics
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub shredstream_connection_status: bool,
    pub rpc_connection_status: bool,
    pub jito_bundle_success_rate: f64,
    pub wallet_balance_sol: f64,

    // Error metrics
    pub circuit_breaker_trips: u64,
    pub failed_rpc_calls: u64,
    pub timeout_errors: u64,
    pub slippage_exceeded_count: u64,

    pub last_updated: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub cooldown_minutes: u32,
    pub enabled: bool,
    pub last_triggered: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    WinRateBelow(f64),
    LatencyAbove(f64),
    ProfitBelow(f64),
    WalletBalanceBelow(f64),
    ErrorRateAbove(f64),
    CircuitBreakerTripped,
    ConnectionLost(String),
    Custom(String, f64), // metric_name, threshold
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub triggered_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
    pub metrics_snapshot: SystemMetrics,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            total_trades: 0,
            successful_trades: 0,
            failed_trades: 0,
            total_profit_sol: 0.0,
            total_loss_sol: 0.0,
            win_rate: 0.0,
            average_latency_ms: 0.0,
            min_latency_ms: f64::MAX,
            max_latency_ms: 0.0,
            throughput_ops_per_sec: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            disk_usage_percent: 0.0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            opportunities_detected: 0,
            opportunities_executed: 0,
            shredstream_connection_status: false,
            rpc_connection_status: false,
            jito_bundle_success_rate: 0.0,
            wallet_balance_sol: 0.0,
            circuit_breaker_trips: 0,
            failed_rpc_calls: 0,
            timeout_errors: 0,
            slippage_exceeded_count: 0,
            last_updated: SystemTime::now(),
        }
    }
}

impl MonitoringSystem {
    pub fn new() -> Self {
        let mut alert_rules = Vec::new();

        // Critical alerts
        alert_rules.push(AlertRule {
            id: "wallet_balance_critical".to_string(),
            name: "Wallet Balance Critical".to_string(),
            condition: AlertCondition::WalletBalanceBelow(0.5),
            severity: AlertSeverity::Critical,
            cooldown_minutes: 5,
            enabled: true,
            last_triggered: None,
        });

        alert_rules.push(AlertRule {
            id: "win_rate_critical".to_string(),
            name: "Win Rate Below Threshold".to_string(),
            condition: AlertCondition::WinRateBelow(0.3),
            severity: AlertSeverity::Critical,
            cooldown_minutes: 10,
            enabled: true,
            last_triggered: None,
        });

        // Warning alerts
        alert_rules.push(AlertRule {
            id: "latency_warning".to_string(),
            name: "High Latency Detected".to_string(),
            condition: AlertCondition::LatencyAbove(50.0),
            severity: AlertSeverity::Warning,
            cooldown_minutes: 5,
            enabled: true,
            last_triggered: None,
        });

        alert_rules.push(AlertRule {
            id: "profit_warning".to_string(),
            name: "Low Profit Performance".to_string(),
            condition: AlertCondition::ProfitBelow(0.01),
            severity: AlertSeverity::Warning,
            cooldown_minutes: 15,
            enabled: true,
            last_triggered: None,
        });

        Self {
            prometheus_client: None,
            grafana_client: None,
            pagerduty_client: None,
            slack_webhook: None,
            discord_webhook: None,
            metrics: Arc::new(Mutex::new(SystemMetrics::default())),
            alert_rules: Arc::new(Mutex::new(alert_rules)),
            active_alerts: Arc::new(Mutex::new(HashMap::new())),
            http_client: Client::new(),
        }
    }

    /// Configure Prometheus integration
    pub fn configure_prometheus(&mut self, endpoint: String, job_name: String, instance: String) {
        self.prometheus_client = Some(PrometheusClient {
            endpoint,
            job_name,
            instance,
        });
        info!("📊 Prometheus monitoring configured");
    }

    /// Configure Grafana integration
    pub fn configure_grafana(&mut self, endpoint: String, api_key: String, dashboard_uid: String) {
        self.grafana_client = Some(GrafanaClient {
            endpoint,
            api_key,
            dashboard_uid,
        });
        info!("📈 Grafana dashboard integration configured");
    }

    /// Configure PagerDuty alerting
    pub fn configure_pagerduty(&mut self, integration_key: String, service_key: String) {
        self.pagerduty_client = Some(PagerDutyClient {
            integration_key,
            service_key,
            api_endpoint: "https://events.pagerduty.com/v2/enqueue".to_string(),
        });
        info!("🚨 PagerDuty alerting configured");
    }

    /// Configure Slack webhook for notifications
    pub fn configure_slack(&mut self, webhook_url: String) {
        self.slack_webhook = Some(webhook_url);
        info!("💬 Slack notifications configured");
    }

    /// Configure Discord webhook for notifications
    pub fn configure_discord(&mut self, webhook_url: String) {
        self.discord_webhook = Some(webhook_url);
        info!("🎮 Discord notifications configured");
    }

    /// Update trading metrics
    pub fn update_trading_metrics(&self, trade_successful: bool, profit_sol: f64, latency_ms: f64) {
        let mut metrics = self.metrics.lock().unwrap();

        metrics.total_trades += 1;
        if trade_successful {
            metrics.successful_trades += 1;
            metrics.total_profit_sol += profit_sol.max(0.0);
        } else {
            metrics.failed_trades += 1;
            metrics.total_loss_sol += profit_sol.abs();
        }

        metrics.win_rate = metrics.successful_trades as f64 / metrics.total_trades as f64;

        // Update latency metrics
        metrics.min_latency_ms = metrics.min_latency_ms.min(latency_ms);
        metrics.max_latency_ms = metrics.max_latency_ms.max(latency_ms);

        // Simple moving average for latency
        let alpha = 0.1;
        metrics.average_latency_ms =
            alpha * latency_ms + (1.0 - alpha) * metrics.average_latency_ms;

        metrics.last_updated = SystemTime::now();

        debug!(
            "📊 Updated trading metrics - Trades: {}, Win rate: {:.2}%, Avg latency: {:.1}ms",
            metrics.total_trades,
            metrics.win_rate * 100.0,
            metrics.average_latency_ms
        );
    }

    /// Update system metrics
    pub fn update_system_metrics(
        &self,
        cpu_percent: f64,
        memory_mb: f64,
        wallet_balance: f64,
        connections_ok: bool,
    ) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.cpu_usage_percent = cpu_percent;
        metrics.memory_usage_mb = memory_mb;
        metrics.wallet_balance_sol = wallet_balance;
        metrics.rpc_connection_status = connections_ok;
        metrics.last_updated = SystemTime::now();
    }

    /// Record an opportunity detection
    pub fn record_opportunity(&self, executed: bool) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.opportunities_detected += 1;
        if executed {
            metrics.opportunities_executed += 1;
        }
    }

    /// Record an error
    pub fn record_error(&self, error_type: &str) {
        let mut metrics = self.metrics.lock().unwrap();
        match error_type {
            "rpc_failure" => metrics.failed_rpc_calls += 1,
            "timeout" => metrics.timeout_errors += 1,
            "slippage" => metrics.slippage_exceeded_count += 1,
            "circuit_breaker" => metrics.circuit_breaker_trips += 1,
            _ => {}
        }
    }

    /// Start monitoring and alerting loop
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🔍 Starting monitoring and alerting system");

        let metrics = Arc::clone(&self.metrics);
        let alert_rules = Arc::clone(&self.alert_rules);
        let active_alerts = Arc::clone(&self.active_alerts);

        // Monitoring loop
        let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds

        loop {
            interval.tick().await;

            // Check alert conditions
            if let Err(e) = self.check_alert_conditions().await {
                error!("❌ Failed to check alert conditions: {}", e);
            }

            // Send metrics to external systems
            if let Err(e) = self.export_metrics().await {
                error!("❌ Failed to export metrics: {}", e);
            }
        }
    }

    /// Check all alert conditions
    async fn check_alert_conditions(&self) -> Result<()> {
        let metrics = self.metrics.lock().unwrap().clone();
        let mut rules = self.alert_rules.lock().unwrap();

        for rule in rules.iter_mut() {
            if !rule.enabled {
                continue;
            }

            // Check cooldown
            if let Some(last_triggered) = rule.last_triggered {
                if last_triggered.elapsed().unwrap_or(Duration::MAX).as_secs()
                    < (rule.cooldown_minutes as u64 * 60)
                {
                    continue;
                }
            }

            let should_trigger = match &rule.condition {
                AlertCondition::WinRateBelow(threshold) => {
                    metrics.win_rate < *threshold && metrics.total_trades > 10
                }
                AlertCondition::LatencyAbove(threshold) => metrics.average_latency_ms > *threshold,
                AlertCondition::ProfitBelow(threshold) => {
                    let total_profit = metrics.total_profit_sol - metrics.total_loss_sol;
                    total_profit < *threshold && metrics.total_trades > 5
                }
                AlertCondition::WalletBalanceBelow(threshold) => {
                    metrics.wallet_balance_sol < *threshold
                }
                AlertCondition::ErrorRateAbove(threshold) => {
                    let error_rate =
                        metrics.failed_trades as f64 / metrics.total_trades.max(1) as f64;
                    error_rate > *threshold
                }
                AlertCondition::CircuitBreakerTripped => metrics.circuit_breaker_trips > 0,
                AlertCondition::ConnectionLost(_) => {
                    !metrics.rpc_connection_status || !metrics.shredstream_connection_status
                }
                AlertCondition::Custom(_, _) => false, // Not implemented
            };

            if should_trigger {
                rule.last_triggered = Some(SystemTime::now());
                let alert = self.create_alert(rule, &metrics).await;
                self.send_alert(&alert).await?;
            }
        }

        Ok(())
    }

    /// Create alert from rule and current metrics
    async fn create_alert(&self, rule: &AlertRule, metrics: &SystemMetrics) -> Alert {
        let alert_id = format!(
            "{}_{}",
            rule.id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let description = match &rule.condition {
            AlertCondition::WinRateBelow(threshold) => format!(
                "Win rate {:.2}% is below threshold {:.2}%",
                metrics.win_rate * 100.0,
                threshold * 100.0
            ),
            AlertCondition::LatencyAbove(threshold) => format!(
                "Average latency {:.1}ms is above threshold {:.1}ms",
                metrics.average_latency_ms, threshold
            ),
            AlertCondition::WalletBalanceBelow(threshold) => format!(
                "Wallet balance {:.6} SOL is below threshold {:.6} SOL",
                metrics.wallet_balance_sol, threshold
            ),
            _ => "Alert condition triggered".to_string(),
        };

        Alert {
            id: alert_id.clone(),
            rule_id: rule.id.clone(),
            title: rule.name.clone(),
            description,
            severity: rule.severity.clone(),
            triggered_at: SystemTime::now(),
            resolved_at: None,
            metrics_snapshot: metrics.clone(),
        }
    }

    /// Send alert through all configured channels
    async fn send_alert(&self, alert: &Alert) -> Result<()> {
        info!(
            "🚨 Triggering alert: {} - {}",
            alert.title, alert.description
        );

        // Store active alert
        {
            let mut active_alerts = self.active_alerts.lock().unwrap();
            active_alerts.insert(alert.id.clone(), alert.clone());
        }

        // Send to PagerDuty
        if let Some(ref pagerduty) = self.pagerduty_client {
            if let Err(e) = self.send_pagerduty_alert(pagerduty, alert).await {
                error!("❌ Failed to send PagerDuty alert: {}", e);
            }
        }

        // Send to Slack
        if let Some(ref webhook) = self.slack_webhook {
            if let Err(e) = self.send_slack_alert(webhook, alert).await {
                error!("❌ Failed to send Slack alert: {}", e);
            }
        }

        // Send to Discord
        if let Some(ref webhook) = self.discord_webhook {
            if let Err(e) = self.send_discord_alert(webhook, alert).await {
                error!("❌ Failed to send Discord alert: {}", e);
            }
        }

        Ok(())
    }

    /// Send alert to PagerDuty
    async fn send_pagerduty_alert(&self, client: &PagerDutyClient, alert: &Alert) -> Result<()> {
        let severity = match alert.severity {
            AlertSeverity::Critical => "critical",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Info => "info",
        };

        let payload = serde_json::json!({
            "routing_key": client.integration_key,
            "event_action": "trigger",
            "payload": {
                "summary": alert.title,
                "source": "mev-bot",
                "severity": severity,
                "custom_details": {
                    "description": alert.description,
                    "wallet_balance": alert.metrics_snapshot.wallet_balance_sol,
                    "win_rate": alert.metrics_snapshot.win_rate,
                    "total_trades": alert.metrics_snapshot.total_trades
                }
            }
        });

        let response = self
            .http_client
            .post(&client.api_endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ PagerDuty alert sent successfully");
        } else {
            warn!(
                "⚠️  PagerDuty alert failed with status: {}",
                response.status()
            );
        }

        Ok(())
    }

    /// Send alert to Slack
    async fn send_slack_alert(&self, webhook: &str, alert: &Alert) -> Result<()> {
        let color = match alert.severity {
            AlertSeverity::Critical => "#ff0000",
            AlertSeverity::Warning => "#ffaa00",
            AlertSeverity::Info => "#0099ff",
        };

        let payload = serde_json::json!({
            "attachments": [{
                "color": color,
                "title": format!("🚨 MEV Bot Alert: {}", alert.title),
                "text": alert.description,
                "fields": [
                    {
                        "title": "Wallet Balance",
                        "value": format!("{:.6} SOL", alert.metrics_snapshot.wallet_balance_sol),
                        "short": true
                    },
                    {
                        "title": "Win Rate",
                        "value": format!("{:.2}%", alert.metrics_snapshot.win_rate * 100.0),
                        "short": true
                    },
                    {
                        "title": "Total Trades",
                        "value": alert.metrics_snapshot.total_trades.to_string(),
                        "short": true
                    },
                    {
                        "title": "Avg Latency",
                        "value": format!("{:.1}ms", alert.metrics_snapshot.average_latency_ms),
                        "short": true
                    }
                ]
            }]
        });

        let response = self
            .http_client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ Slack alert sent successfully");
        }

        Ok(())
    }

    /// Send alert to Discord
    async fn send_discord_alert(&self, webhook: &str, alert: &Alert) -> Result<()> {
        let color = match alert.severity {
            AlertSeverity::Critical => 0xff0000,
            AlertSeverity::Warning => 0xffaa00,
            AlertSeverity::Info => 0x0099ff,
        };

        let payload = serde_json::json!({
            "embeds": [{
                "title": format!("🚨 MEV Bot Alert: {}", alert.title),
                "description": alert.description,
                "color": color,
                "fields": [
                    {
                        "name": "Wallet Balance",
                        "value": format!("{:.6} SOL", alert.metrics_snapshot.wallet_balance_sol),
                        "inline": true
                    },
                    {
                        "name": "Win Rate",
                        "value": format!("{:.2}%", alert.metrics_snapshot.win_rate * 100.0),
                        "inline": true
                    },
                    {
                        "name": "Total Trades",
                        "value": alert.metrics_snapshot.total_trades.to_string(),
                        "inline": true
                    }
                ]
            }]
        });

        let response = self
            .http_client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ Discord alert sent successfully");
        }

        Ok(())
    }

    /// Export metrics to external systems
    async fn export_metrics(&self) -> Result<()> {
        let metrics = self.metrics.lock().unwrap().clone();

        // Export to Prometheus
        if let Some(ref prometheus) = self.prometheus_client {
            if let Err(e) = self.export_to_prometheus(prometheus, &metrics).await {
                error!("❌ Failed to export to Prometheus: {}", e);
            }
        }

        Ok(())
    }

    /// Export metrics to Prometheus
    async fn export_to_prometheus(
        &self,
        client: &PrometheusClient,
        metrics: &SystemMetrics,
    ) -> Result<()> {
        let prometheus_metrics = format!(
            "# HELP mev_bot_trades_total Total number of trades executed\n\
             # TYPE mev_bot_trades_total counter\n\
             mev_bot_trades_total{{job=\"{}\",instance=\"{}\"}} {}\n\
             # HELP mev_bot_win_rate Current win rate percentage\n\
             # TYPE mev_bot_win_rate gauge\n\
             mev_bot_win_rate{{job=\"{}\",instance=\"{}\"}} {}\n\
             # HELP mev_bot_latency_ms Average latency in milliseconds\n\
             # TYPE mev_bot_latency_ms gauge\n\
             mev_bot_latency_ms{{job=\"{}\",instance=\"{}\"}} {}\n\
             # HELP mev_bot_wallet_balance_sol Wallet balance in SOL\n\
             # TYPE mev_bot_wallet_balance_sol gauge\n\
             mev_bot_wallet_balance_sol{{job=\"{}\",instance=\"{}\"}} {}\n",
            client.job_name,
            client.instance,
            metrics.total_trades,
            client.job_name,
            client.instance,
            metrics.win_rate,
            client.job_name,
            client.instance,
            metrics.average_latency_ms,
            client.job_name,
            client.instance,
            metrics.wallet_balance_sol
        );

        let response = self
            .http_client
            .post(&format!(
                "{}/metrics/job/{}/instance/{}",
                client.endpoint, client.job_name, client.instance
            ))
            .header("Content-Type", "text/plain")
            .body(prometheus_metrics)
            .send()
            .await?;

        if response.status().is_success() {
            debug!("✅ Metrics exported to Prometheus successfully");
        }

        Ok(())
    }

    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> SystemMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }
}

impl Default for MonitoringSystem {
    fn default() -> Self {
        Self::new()
    }
}
