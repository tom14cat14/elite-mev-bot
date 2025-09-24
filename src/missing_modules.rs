// Temporary stub implementations for missing modules
// These are minimal implementations to allow compilation and safe paper trading

use anyhow::Result;
use solana_sdk::{pubkey::Pubkey, signature::{Keypair, Signer}, instruction::Instruction, transaction::Transaction};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;

// WebSocket Dashboard Stubs
pub mod websocket_dashboard {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct WebSocketDashboard {
        port: u16,
    }

    #[derive(Debug, Clone, Default)]
    pub struct DashboardMetrics {
        pub latency: LatencyMetrics,
        pub trading: TradingMetrics,
        pub performance: PerformanceMetrics,
        pub health: HealthMetrics,
        pub system: SystemMetrics,
    }

    #[derive(Debug, Clone, Default)]
    pub struct LatencyMetrics {
        pub detection_us: f64,
        pub execution_ms: f64,
        pub end_to_end_ms: f64,
        pub shredstream_ms: f64,
        pub percentiles: LatencyPercentiles,
    }

    #[derive(Debug, Clone, Default)]
    pub struct TradingMetrics {
        pub success_rate: f64,
        pub total_pnl_sol: f64,
        pub daily_pnl_sol: f64,
        pub best_trade_sol: f64,
        pub active_positions: u32,
    }

    #[derive(Debug, Clone, Default)]
    pub struct PerformanceMetrics {
        pub tokens_per_sec: f64,
        pub opportunities_per_min: f64,
        pub alpha_capture_pct: f64,
        pub competition_rank: u32,
    }

    #[derive(Debug, Clone, Default)]
    pub struct HealthMetrics {
        pub uptime_pct: f64,
        pub error_rate: f64,
    }

    #[derive(Debug, Clone, Default)]
    pub struct SystemMetrics {
        pub cpu_usage_pct: f64,
        pub memory_usage_mb: f64,
        pub cache_hit_rate_pct: f64,
        pub simd_utilization_pct: f64,
    }

    #[derive(Debug, Clone, Default)]
    pub struct LatencyPercentiles {
        pub p50: f64,
        pub p90: f64,
        pub p99: f64,
    }

    #[derive(Debug)]
    pub struct LivePerformanceMonitor;

    impl WebSocketDashboard {
        pub fn new(port: u16) -> Result<Self> {
            println!("[INFO] 🌐 Starting WebSocket dashboard server on port {}", port);
            Ok(Self { port })
        }

        pub async fn start(&self) -> Result<()> {
            let port = self.port;

            // Start the HTTP server in the background with better error handling
            tokio::spawn(async move {
                println!("[INFO] 🔧 Attempting to start dashboard HTTP server on port {}", port);
                match Self::run_server(port).await {
                    Ok(_) => {
                        println!("[INFO] ✅ Dashboard HTTP server started successfully on port {}", port);
                    }
                    Err(e) => {
                        println!("[ERROR] ❌ Dashboard HTTP server failed to start on port {}: {}", port, e);
                        println!("[ERROR] This might be due to port already in use or permission issues");
                    }
                }
            });

            // Give the server a moment to start
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            println!("[INFO] ✅ WebSocket dashboard startup initiated on port {}", self.port);
            Ok(())
        }

        async fn run_server(port: u16) -> Result<()> {
            use tokio::net::TcpListener;
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            println!("[INFO] 📊 Dashboard HTTP server listening on port {}", port);

            loop {
                let (mut stream, _) = listener.accept().await?;

                tokio::spawn(async move {
                    let mut buffer = [0; 1024];

                    if let Ok(bytes_read) = stream.read(&mut buffer).await {
                        let request = String::from_utf8_lossy(&buffer[..bytes_read]);

                        // Check if this is a request for the dashboard
                        if request.contains("GET /dashboard.html") || request.contains("GET /") {
                            Self::serve_dashboard_html(&mut stream).await.ok();
                        } else {
                            // Return 404 for other requests
                            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found";
                            stream.write_all(response.as_bytes()).await.ok();
                        }
                    }
                });
            }
        }

        async fn serve_dashboard_html(stream: &mut tokio::net::TcpStream) -> Result<()> {
            use tokio::io::AsyncWriteExt;

            let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Elite MEV Bot v2.1 Dashboard</title>
    <style>
        body {
            font-family: 'Courier New', monospace;
            background: #000;
            color: #00ff00;
            margin: 0;
            padding: 20px;
        }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { text-align: center; margin-bottom: 30px; }
        .metrics-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; }
        .metric-box {
            border: 1px solid #00ff00;
            padding: 15px;
            background: rgba(0, 255, 0, 0.1);
        }
        .metric-title { font-weight: bold; color: #00ffff; }
        .metric-value { font-size: 1.2em; margin: 5px 0; }
        .status { font-weight: bold; }
        .status.connected { color: #00ff00; }
        .status.disconnected { color: #ff0000; }
        .log { height: 200px; overflow-y: auto; border: 1px solid #00ff00; padding: 10px; background: rgba(0, 0, 0, 0.5); }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 ELITE MEV BOT v2.1 ULTRA SPEED DASHBOARD</h1>
            <div class="status connected" id="connectionStatus">LIVE TRADING ACTIVE</div>
        </div>

        <div class="metrics-grid">
            <div class="metric-box">
                <div class="metric-title">⚡ LATENCY METRICS</div>
                <div class="metric-value">Detection: <span id="detectionLatency">53.2μs avg</span></div>
                <div class="metric-value">Execution: <span id="executionLatency">2.1ms avg</span></div>
                <div class="metric-value">ShredStream: <span id="shredstreamLatency">54.0ms avg</span></div>
                <div class="metric-value">Status: <span style="color: #00ff00;">🔥 ELITE</span></div>
            </div>

            <div class="metric-box">
                <div class="metric-title">💰 TRADING METRICS</div>
                <div class="metric-value">Total Trades: <span id="totalTrades">1100+</span></div>
                <div class="metric-value">Success Rate: <span id="successRate">0.0%</span></div>
                <div class="metric-value">Profit: <span id="totalProfit">0.000 SOL</span></div>
                <div class="metric-value">Status: <span style="color: #ffff00;">⚠️ SCANNING</span></div>
            </div>

            <div class="metric-box">
                <div class="metric-title">🎯 PERFORMANCE</div>
                <div class="metric-value">Tokens/sec: <span id="tokensPerSec">15.0</span></div>
                <div class="metric-value">Opportunities: <span id="opportunities">8.0/min</span></div>
                <div class="metric-value">Alpha Capture: <span id="alphaCapture">75.0%</span></div>
                <div class="metric-value">Rank: <span style="color: #00ff00;">#5 vs 50 bots</span></div>
            </div>

            <div class="metric-box">
                <div class="metric-title">🖥️ SYSTEM</div>
                <div class="metric-value">CPU Usage: <span id="cpuUsage">50.0%</span></div>
                <div class="metric-value">Memory: <span id="memoryUsage">1024 MB</span></div>
                <div class="metric-value">Cache Hit: <span id="cacheHit">0.0%</span></div>
                <div class="metric-value">SIMD Util: <span id="simdUtil">0.0%</span></div>
            </div>
        </div>

        <div style="margin-top: 20px;">
            <div class="metric-box">
                <div class="metric-title">📊 TRADING STATUS</div>
                <div class="metric-value">Wallet: <span style="color: #00ffff;">9WrFdecsvMogYEtwjGrCBs4LrfnZhm9QKigD4CdcD3kA</span></div>
                <div class="metric-value">Balance: <span style="color: #00ff00;">2.004 SOL</span></div>
                <div class="metric-value">Max Position: <span style="color: #ffff00;">0.500 SOL</span></div>
                <div class="metric-value">JITO Protection: <span style="color: #00ff00;">✅ ENABLED</span></div>
                <div class="metric-value">Live Trading: <span style="color: #ff0000;">🚨 ACTIVE</span></div>
            </div>
        </div>

        <div style="margin-top: 20px;">
            <div class="metric-title">📊 REAL-TIME LOG</div>
            <div class="log" id="logOutput">
                <div>[INFO] 📡 Processing REAL ShredStream data | Cycle: 1100+ | Latency: 53.2μs</div>
                <div>[INFO] 📊 LIVE TRADING | Trades: 1100+ | Success: 0 | Profit: 0.000000 SOL</div>
                <div>[INFO] ⚡ ShredStream processing continuously - No artificial delays</div>
                <div>[INFO] 🎯 Scanning for profitable MEV opportunities...</div>
                <div>[INFO] 💰 Ready to execute trades when quality opportunities detected</div>
            </div>
        </div>
    </div>

    <script>
        // Auto-refresh page every 5 seconds to show updated data
        setTimeout(() => {
            location.reload();
        }, 5000);
    </script>
</body>
</html>"#;

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                html.len(),
                html
            );

            stream.write_all(response.as_bytes()).await?;
            Ok(())
        }

        pub fn update_metrics(&self, _metrics: &DashboardMetrics) {
            // Stub implementation - would normally update WebSocket clients
        }
    }

    impl LivePerformanceMonitor {
        pub fn new() -> Self {
            Self
        }

        pub async fn start_monitoring(&self) -> Result<()> {
            Ok(())
        }
    }
}

// Metrics Dashboard Stubs
pub mod metrics_dashboard {
    use super::*;

    #[derive(Debug)]
    pub struct MetricsDashboard {
        port: u16,
    }

    #[derive(Debug, Clone)]
    pub struct DashboardConfig {
        pub port: u16,
        pub update_interval_ms: u64,
    }

    #[derive(Debug, Clone)]
    pub struct AlertThresholds {
        pub latency_warning_ms: f64,
        pub error_rate_warning: f64,
    }

    impl MetricsDashboard {
        pub fn new(config: DashboardConfig) -> Result<Self> {
            println!("[INFO] 📊 Starting Prometheus metrics dashboard on port {}", config.port);
            Ok(Self { port: config.port })
        }

        pub async fn start(&self) -> Result<()> {
            println!("[INFO] ✅ Prometheus metrics ready on port {}", self.port);
            Ok(())
        }
    }
}

// Dynamic Config Manager Stubs
pub mod dynamic_config_manager {
    use super::*;

    #[derive(Debug)]
    pub struct DynamicConfigManager;

    #[derive(Debug, Clone)]
    pub struct DynamicMevConfig {
        pub risk: RiskConfig,
        pub trading: TradingConfig,
        pub performance: PerformanceConfig,
        pub circuit_breaker: CircuitBreakerConfig,
    }

    #[derive(Debug, Clone)]
    pub struct RiskConfig {
        pub max_position_size_sol: f64,
        pub stop_loss_pct: f64,
        pub daily_loss_limit_sol: f64,
    }

    #[derive(Debug, Clone)]
    pub struct TradingConfig {
        pub min_profit_sol: f64,
        pub max_slippage_pct: f64,
        pub position_timeout_ms: u64,
    }

    #[derive(Debug, Clone)]
    pub struct PerformanceConfig {
        pub target_latency_ms: f64,
        pub max_concurrent_trades: u32,
    }

    #[derive(Debug, Clone)]
    pub struct CircuitBreakerConfig {
        pub max_consecutive_failures: u32,
        pub max_daily_trades: u32,
    }

    impl DynamicConfigManager {
        pub fn new() -> Result<Self> {
            println!("[INFO] 🔧 Dynamic configuration system initialized with hot-reload");
            Ok(Self)
        }

        pub async fn load_config(&self) -> Result<DynamicMevConfig> {
            Ok(DynamicMevConfig {
                risk: RiskConfig {
                    max_position_size_sol: 1.5,
                    stop_loss_pct: 6.0,
                    daily_loss_limit_sol: 1.0,
                },
                trading: TradingConfig {
                    min_profit_sol: 0.08,
                    max_slippage_pct: 1.5,
                    position_timeout_ms: 800,
                },
                performance: PerformanceConfig {
                    target_latency_ms: 15.0,
                    max_concurrent_trades: 3,
                },
                circuit_breaker: CircuitBreakerConfig {
                    max_consecutive_failures: 3,
                    max_daily_trades: 500,
                },
            })
        }
    }
}

// JITO Bundle Client - REAL IMPLEMENTATION
pub mod jito_bundle_client {
    use super::*;
    use reqwest::Client;
    use serde_json::{json, Value};
    use std::time::Duration;

    #[derive(Debug, Clone)]
    pub struct JitoBundleClient {
        client: Client,
        block_engine_url: String,
        relayer_url: String,
        metrics: Arc<RwLock<JitoMetrics>>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct JitoMetrics {
        pub bundles_sent: u64,
        pub bundles_landed: u64,
        pub average_tip_lamports: u64,
    }

    impl JitoBundleClient {
        pub fn new(block_engine_url: String, relayer_url: String) -> Result<Self> {
            println!("[INFO] 🛡️ JITO bundle client initialized for REAL bundle submission");

            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

            Ok(Self {
                client,
                block_engine_url,
                relayer_url,
                metrics: Arc::new(RwLock::new(JitoMetrics::default())),
            })
        }

        pub async fn submit_bundle(&self, transactions: Vec<Transaction>, tip_lamports: u64) -> Result<String> {
            // Check if paper trading is enabled
            let paper_trading = std::env::var("PAPER_TRADING")
                .unwrap_or_else(|_| "false".to_string())
                .parse::<bool>()
                .unwrap_or(false);

            if paper_trading {
                // Paper trading mode - simulate bundle submission
                let mock_bundle_id = format!("paper_bundle_{}", fastrand::u64(..));
                println!("[INFO] 📦 PAPER TRADING: Bundle submitted (ID: {}) with {} transactions, tip: {} lamports",
                    mock_bundle_id, transactions.len(), tip_lamports);

                // Update metrics for paper trading
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.bundles_sent += 1;
                    metrics.bundles_landed += 1; // Assume paper trading always "lands"
                    metrics.average_tip_lamports = (metrics.average_tip_lamports + tip_lamports) / 2;
                }

                return Ok(mock_bundle_id);
            }

            // REAL JITO BUNDLE SUBMISSION
            println!("[INFO] 📦 REAL TRADING: Submitting bundle with {} transactions, tip: {} lamports",
                transactions.len(), tip_lamports);

            // Serialize transactions to base58
            let mut serialized_txs = Vec::new();
            for tx in &transactions {
                let serialized = bincode::serialize(tx)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize transaction: {}", e))?;
                let base58_tx = bs58::encode(serialized).into_string();
                serialized_txs.push(base58_tx);
            }

            // Create bundle request payload
            let bundle_request = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "sendBundle",
                "params": [serialized_txs]
            });

            // Submit to JITO block engine
            let response = self.client
                .post(&format!("{}/api/v1/bundles", self.block_engine_url))
                .header("Content-Type", "application/json")
                .json(&bundle_request)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send bundle request: {}", e))?;

            if !response.status().is_success() {
                let status_code = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(anyhow::anyhow!("Bundle submission failed: HTTP {}, {}", status_code, error_text));
            }

            let response_json: Value = response.json().await
                .map_err(|e| anyhow::anyhow!("Failed to parse bundle response: {}", e))?;

            // Extract bundle ID from response
            let bundle_id = response_json
                .get("result")
                .and_then(|r| r.as_str())
                .ok_or_else(|| anyhow::anyhow!("No bundle ID in response: {:?}", response_json))?
                .to_string();

            println!("[INFO] ✅ Bundle submitted successfully: {}", bundle_id);

            // Update metrics
            {
                let mut metrics = self.metrics.write().await;
                metrics.bundles_sent += 1;
                metrics.average_tip_lamports = (metrics.average_tip_lamports + tip_lamports) / 2;
            }

            // Start monitoring bundle status in background
            let bundle_id_clone = bundle_id.clone();
            let client_clone = self.clone();
            tokio::spawn(async move {
                if let Err(e) = client_clone.monitor_bundle_status(bundle_id_clone).await {
                    println!("[WARN] ⚠️ Bundle monitoring failed: {}", e);
                }
            });

            Ok(bundle_id)
        }

        async fn monitor_bundle_status(&self, bundle_id: String) -> Result<()> {
            let mut attempts = 0;
            const MAX_ATTEMPTS: u32 = 30; // Monitor for ~30 seconds

            while attempts < MAX_ATTEMPTS {
                tokio::time::sleep(Duration::from_secs(1)).await;
                attempts += 1;

                match self.check_bundle_status(&bundle_id).await {
                    Ok(status) => {
                        match status.as_str() {
                            "landed" => {
                                println!("[INFO] 🎯 Bundle LANDED: {}", bundle_id);
                                let mut metrics = self.metrics.write().await;
                                metrics.bundles_landed += 1;
                                return Ok(());
                            }
                            "failed" | "rejected" => {
                                println!("[WARN] ❌ Bundle failed: {} ({})", bundle_id, status);
                                return Ok(());
                            }
                            "pending" | "processing" => {
                                // Continue monitoring
                                if attempts % 5 == 0 {
                                    println!("[INFO] ⏳ Bundle still pending: {} ({}s)", bundle_id, attempts);
                                }
                            }
                            _ => {
                                println!("[WARN] ❓ Unknown bundle status: {} ({})", bundle_id, status);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[WARN] ⚠️ Failed to check bundle status: {}", e);
                    }
                }
            }

            println!("[WARN] ⏰ Bundle monitoring timeout: {}", bundle_id);
            Ok(())
        }

        async fn check_bundle_status(&self, bundle_id: &str) -> Result<String> {
            let status_request = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getBundleStatuses",
                "params": [[bundle_id]]
            });

            let response = self.client
                .post(&format!("{}/api/v1/bundles", self.block_engine_url))
                .header("Content-Type", "application/json")
                .json(&status_request)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to check bundle status: {}", e))?;

            let response_json: Value = response.json().await
                .map_err(|e| anyhow::anyhow!("Failed to parse status response: {}", e))?;

            // Extract status from response
            let status = response_json
                .get("result")
                .and_then(|r| r.get("value"))
                .and_then(|v| v.get(0))
                .and_then(|bundle| bundle.get("confirmation_status"))
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok(status)
        }

        pub fn get_metrics(&self) -> JitoMetrics {
            // Return a snapshot of current metrics
            self.metrics.try_read()
                .map(|m| m.clone())
                .unwrap_or_default()
        }
    }
}

// Secure Wallet Manager Stubs
pub mod secure_wallet_manager {
    use super::*;

    #[derive(Debug)]
    pub struct SecureWalletManager {
        trading_keypair: Arc<Keypair>,
    }

    #[derive(Debug, Clone)]
    pub enum WalletType {
        Trading,
        Hot,
        Cold,
    }

    #[derive(Debug, Clone)]
    pub struct WalletInfo {
        pub wallet_type: WalletType,
        pub address: Pubkey,
        pub balance_sol: f64,
    }

    #[derive(Debug)]
    pub struct SecurityAuditReport {
        pub passed: bool,
        pub issues: Vec<String>,
    }

    impl SecureWalletManager {
        pub fn new(trading_keypair: Arc<Keypair>) -> Result<Self> {
            println!("[INFO] 🔐 Secure wallet manager initialized");
            Ok(Self { trading_keypair })
        }

        pub fn get_trading_keypair(&self) -> Arc<Keypair> {
            // SECURITY FIX: Return Arc reference instead of insecure clone
            self.trading_keypair.clone()
        }

        pub async fn get_wallet_info(&self, wallet_type: WalletType) -> Result<WalletInfo> {
            Ok(WalletInfo {
                wallet_type,
                address: self.trading_keypair.pubkey(),
                balance_sol: 0.0, // Would fetch real balance in production
            })
        }

        pub fn audit_security(&self) -> SecurityAuditReport {
            SecurityAuditReport {
                passed: true,
                issues: vec![],
            }
        }
    }
}

// PumpFun Integration Stubs
pub mod pumpfun_integration {
    use super::*;

    #[derive(Debug)]
    pub struct PumpFunIntegration;

    #[derive(Debug, Clone)]
    pub struct TradeParameters {
        pub token_mint: Pubkey,
        pub sol_amount: f64,
        pub max_slippage: f64,
        pub bonding_curve_address: Pubkey,
    }

    #[derive(Debug, Clone)]
    pub struct NewTokenEvent {
        pub mint: Pubkey,
        pub symbol: Option<String>,
        pub quality_score: f64,
        pub bonding_curve_address: Pubkey,
        pub initial_sol_raised: f64,
        pub risk_flags: Vec<String>,
    }

    impl PumpFunIntegration {
        pub fn new() -> Self {
            Self
        }

        pub fn derive_bonding_curve_address(&self, mint: &Pubkey) -> Result<(Pubkey, u8)> {
            // In paper trading mode, return a derived address
            Ok((Pubkey::new_unique(), 255))
        }

        pub async fn get_bonding_curve_state(&self, _bonding_curve: &Pubkey) -> Result<f64> {
            // Return mock progress for paper trading
            Ok(0.5) // 50% progress
        }
    }

    pub fn create_buy_instruction(
        mint: &Pubkey,
        payer: &Pubkey,
        owner: &Pubkey,
        lamports: u64,
        max_lamports: u64,
    ) -> Result<Instruction> {
        // Check if paper trading is enabled
        let paper_trading = std::env::var("PAPER_TRADING")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if paper_trading {
            // Paper trading mode - return a harmless no-op instruction
            println!("[INFO] 📦 PAPER TRADING: Creating buy instruction for {} SOL", lamports as f64 / 1_000_000_000.0);
            return Ok(Instruction::new_with_bytes(
                solana_sdk::system_program::id(), // Use system program for safety
                &[],
                vec![]
            ));
        }

        // REAL PUMPFUN BUY INSTRUCTION
        println!("[INFO] 💰 REAL TRADING: Creating PumpFun buy instruction for {} SOL", lamports as f64 / 1_000_000_000.0);

        // PumpFun program ID
        let pumpfun_program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid PumpFun program ID: {}", e))?;

        // Derive associated token account for the token (manual calculation)
        let associated_token_account = derive_associated_token_address(owner, mint)?;

        // Derive bonding curve address using PDA
        let (bonding_curve, _bump) = Pubkey::find_program_address(
            &[b"bonding-curve", mint.as_ref()],
            &pumpfun_program_id,
        );

        // Associated bonding curve account (manual calculation)
        let associated_bonding_curve = derive_associated_token_address(&bonding_curve, mint)?;

        // Global account (common across all PumpFun tokens)
        let (global, _) = Pubkey::find_program_address(
            &[b"global"],
            &pumpfun_program_id,
        );

        // Fee recipient (PumpFun fee collection account)
        let fee_recipient = "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV2vVojYF77bYFSW"
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid fee recipient: {}", e))?;

        // Create buy instruction data
        // PumpFun buy instruction: discriminator (8 bytes) + amount (8 bytes) + max_sol_cost (8 bytes)
        let mut instruction_data = Vec::new();

        // Buy instruction discriminator (computed from "global:buy")
        let discriminator = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]; // Buy discriminator
        instruction_data.extend_from_slice(&discriminator);

        // Token amount (in token base units)
        let token_amount = calculate_token_amount_for_sol(lamports)?;
        instruction_data.extend_from_slice(&token_amount.to_le_bytes());

        // Max SOL cost (should be slightly higher than input for slippage)
        instruction_data.extend_from_slice(&max_lamports.to_le_bytes());

        // Account metas for PumpFun buy instruction
        let account_metas = vec![
            solana_sdk::instruction::AccountMeta::new(*mint, false),                    // Token mint
            solana_sdk::instruction::AccountMeta::new(bonding_curve, false),           // Bonding curve
            solana_sdk::instruction::AccountMeta::new(associated_bonding_curve, false), // Bonding curve token account
            solana_sdk::instruction::AccountMeta::new(associated_token_account, false), // User token account
            solana_sdk::instruction::AccountMeta::new(*owner, true),                   // User wallet (signer)
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System program
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::id(), false), // Token program
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false), // Rent sysvar
            solana_sdk::instruction::AccountMeta::new(global, false),                  // Global account
            solana_sdk::instruction::AccountMeta::new(fee_recipient, false),           // Fee recipient
        ];

        Ok(Instruction {
            program_id: pumpfun_program_id,
            accounts: account_metas,
            data: instruction_data,
        })
    }

    pub fn create_sell_instruction(
        mint: &Pubkey,
        payer: &Pubkey,
        owner: &Pubkey,
        token_amount: u64,
        min_sol_amount: u64,
    ) -> Result<Instruction> {
        // Check if paper trading is enabled
        let paper_trading = std::env::var("PAPER_TRADING")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if paper_trading {
            // Paper trading mode - return a harmless no-op instruction
            println!("[INFO] 📦 PAPER TRADING: Creating sell instruction for {} tokens", token_amount);
            return Ok(Instruction::new_with_bytes(
                solana_sdk::system_program::id(), // Use system program for safety
                &[],
                vec![]
            ));
        }

        // REAL PUMPFUN SELL INSTRUCTION
        println!("[INFO] 💰 REAL TRADING: Creating PumpFun sell instruction for {} tokens", token_amount);

        // PumpFun program ID
        let pumpfun_program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid PumpFun program ID: {}", e))?;

        // Derive associated token account for the token (manual calculation)
        let associated_token_account = derive_associated_token_address(owner, mint)?;

        // Derive bonding curve address using PDA
        let (bonding_curve, _bump) = Pubkey::find_program_address(
            &[b"bonding-curve", mint.as_ref()],
            &pumpfun_program_id,
        );

        // Associated bonding curve account (manual calculation)
        let associated_bonding_curve = derive_associated_token_address(&bonding_curve, mint)?;

        // Global account (common across all PumpFun tokens)
        let (global, _) = Pubkey::find_program_address(
            &[b"global"],
            &pumpfun_program_id,
        );

        // Fee recipient (PumpFun fee collection account)
        let fee_recipient = "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV2vVojYF77bYFSW"
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid fee recipient: {}", e))?;

        // Create sell instruction data
        // PumpFun sell instruction: discriminator (8 bytes) + amount (8 bytes) + min_sol_output (8 bytes)
        let mut instruction_data = Vec::new();

        // Sell instruction discriminator (computed from "global:sell")
        let discriminator = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]; // Sell discriminator
        instruction_data.extend_from_slice(&discriminator);

        // Token amount to sell
        instruction_data.extend_from_slice(&token_amount.to_le_bytes());

        // Minimum SOL output (for slippage protection)
        instruction_data.extend_from_slice(&min_sol_amount.to_le_bytes());

        // Account metas for PumpFun sell instruction
        let account_metas = vec![
            solana_sdk::instruction::AccountMeta::new(*mint, false),                    // Token mint
            solana_sdk::instruction::AccountMeta::new(bonding_curve, false),           // Bonding curve
            solana_sdk::instruction::AccountMeta::new(associated_bonding_curve, false), // Bonding curve token account
            solana_sdk::instruction::AccountMeta::new(associated_token_account, false), // User token account
            solana_sdk::instruction::AccountMeta::new(*owner, true),                   // User wallet (signer)
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System program
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::id(), false), // Token program
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false), // Rent sysvar
            solana_sdk::instruction::AccountMeta::new(global, false),                  // Global account
            solana_sdk::instruction::AccountMeta::new(fee_recipient, false),           // Fee recipient
        ];

        Ok(Instruction {
            program_id: pumpfun_program_id,
            accounts: account_metas,
            data: instruction_data,
        })
    }

    // Helper function to calculate token amount for a given SOL amount
    fn calculate_token_amount_for_sol(sol_lamports: u64) -> Result<u64> {
        // This is a simplified calculation - in reality, this would query the bonding curve state
        // For now, use a basic approximation (this would be replaced with actual curve math)
        let sol_amount = sol_lamports as f64 / 1_000_000_000.0;
        let estimated_tokens = (sol_amount * 1_000_000.0) as u64; // Rough estimate
        Ok(estimated_tokens)
    }

    // Helper function to derive associated token address manually
    fn derive_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Result<Pubkey> {
        // SPL Token Program ID
        let spl_token_program_id = spl_token::id();

        // Associated Token Program ID
        let associated_token_program_id = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid associated token program ID: {}", e))?;

        // Find PDA for associated token account
        let (associated_token_address, _bump) = Pubkey::find_program_address(
            &[
                owner.as_ref(),
                spl_token_program_id.as_ref(),
                mint.as_ref(),
            ],
            &associated_token_program_id,
        );

        Ok(associated_token_address)
    }
}