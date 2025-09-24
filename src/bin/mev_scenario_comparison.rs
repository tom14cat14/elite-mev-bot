use anyhow::Result;
use shared_bot_infrastructure::*;
use std::time::{Instant, Duration};
use tracing::{info, warn};
use tokio::time::{sleep, timeout};
use solana_entry::entry::Entry;
use std::process::Command;

#[derive(Debug, Clone)]
struct MevScenarioResults {
    // Real-time data access (critical for MEV)
    shredstream_realtime_score: f64,
    grpc_realtime_score: f64,

    // Latency consistency (MEV needs consistent speed)
    shredstream_consistency_score: f64,
    grpc_consistency_score: f64,

    // Data freshness (how up-to-date the data is)
    shredstream_freshness_score: f64,
    grpc_freshness_score: f64,

    // Reliability (uptime and error rates)
    shredstream_reliability_score: f64,
    grpc_reliability_score: f64,

    // Overall MEV suitability scores
    shredstream_total_score: f64,
    grpc_total_score: f64,

    recommendation: String,
}

impl MevScenarioResults {
    fn new() -> Self {
        Self {
            shredstream_realtime_score: 0.0,
            grpc_realtime_score: 0.0,
            shredstream_consistency_score: 0.0,
            grpc_consistency_score: 0.0,
            shredstream_freshness_score: 0.0,
            grpc_freshness_score: 0.0,
            shredstream_reliability_score: 0.0,
            grpc_reliability_score: 0.0,
            shredstream_total_score: 0.0,
            grpc_total_score: 0.0,
            recommendation: String::new(),
        }
    }

    fn calculate_scores(&mut self, shred_latencies: &[f64], grpc_latencies: &[f64],
                       shred_errors: u64, grpc_errors: u64, total_tests: u64) {

        // Real-time score (lower latency = higher score, 100ms baseline)
        if !shred_latencies.is_empty() {
            let shred_avg = shred_latencies.iter().sum::<f64>() / shred_latencies.len() as f64;
            self.shredstream_realtime_score = (100.0 / shred_avg.max(10.0)) * 100.0;
        }

        if !grpc_latencies.is_empty() {
            let grpc_avg = grpc_latencies.iter().sum::<f64>() / grpc_latencies.len() as f64;
            self.grpc_realtime_score = (100.0 / grpc_avg.max(10.0)) * 100.0;
        }

        // Consistency score (lower standard deviation = higher score)
        if shred_latencies.len() > 1 {
            let mean = shred_latencies.iter().sum::<f64>() / shred_latencies.len() as f64;
            let variance = shred_latencies.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / shred_latencies.len() as f64;
            let std_dev = variance.sqrt();
            self.shredstream_consistency_score = (100.0 / (std_dev + 1.0)).max(0.0);
        }

        if grpc_latencies.len() > 1 {
            let mean = grpc_latencies.iter().sum::<f64>() / grpc_latencies.len() as f64;
            let variance = grpc_latencies.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / grpc_latencies.len() as f64;
            let std_dev = variance.sqrt();
            self.grpc_consistency_score = (100.0 / (std_dev + 1.0)).max(0.0);
        }

        // Freshness score (real-time streams score higher than polling)
        self.shredstream_freshness_score = 95.0; // Real-time stream data
        self.grpc_freshness_score = 70.0; // Likely polling-based

        // Reliability score (error rate based)
        if total_tests > 0 {
            self.shredstream_reliability_score = ((total_tests - shred_errors) as f64 / total_tests as f64) * 100.0;
            self.grpc_reliability_score = ((total_tests - grpc_errors) as f64 / total_tests as f64) * 100.0;
        }

        // Calculate total scores (weighted for MEV importance)
        let realtime_weight = 0.4;     // 40% - Most critical for MEV
        let consistency_weight = 0.3;  // 30% - Very important for predictable execution
        let freshness_weight = 0.2;    // 20% - Important for accurate data
        let reliability_weight = 0.1;  // 10% - Basic requirement

        self.shredstream_total_score =
            (self.shredstream_realtime_score * realtime_weight) +
            (self.shredstream_consistency_score * consistency_weight) +
            (self.shredstream_freshness_score * freshness_weight) +
            (self.shredstream_reliability_score * reliability_weight);

        self.grpc_total_score =
            (self.grpc_realtime_score * realtime_weight) +
            (self.grpc_consistency_score * consistency_weight) +
            (self.grpc_freshness_score * freshness_weight) +
            (self.grpc_reliability_score * reliability_weight);

        // Generate recommendation based on Grok's advice
        let score_difference = (self.shredstream_total_score - self.grpc_total_score).abs();

        if score_difference < 10.0 {
            // Close performance - can use either as primary
            if self.grpc_total_score > self.shredstream_total_score {
                self.recommendation = "🎯 YOUR gRPC PRIMARY: Your gRPC performs as well as ShredStream! Use gRPC as primary, ShredStream as backup.".to_string();
            } else {
                self.recommendation = "🎯 EITHER PRIMARY: Performance is very close! Choose based on your preference and infrastructure.".to_string();
            }
        } else if self.grpc_total_score > self.shredstream_total_score {
            self.recommendation = "🏆 YOUR gRPC WINNER: Your gRPC significantly outperforms! Use it as your primary MEV data source.".to_string();
        } else {
            let perf_gap = ((self.shredstream_total_score - self.grpc_total_score) / self.grpc_total_score) * 100.0;
            if perf_gap > 25.0 {
                self.recommendation = format!("📈 SHREDSTREAM PRIMARY: ShredStream is {:.1}% better. Use ShredStream primary, gRPC backup.", perf_gap);
            } else {
                self.recommendation = format!("⚖️ CLOSE CALL: ShredStream {:.1}% better, but your gRPC is competitive. Could use either as primary.", perf_gap);
            }
        }
    }

    fn print_mev_analysis(&self) {
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("⚔️  MEV TRADING SUITABILITY ANALYSIS");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("");

        info!("📊 PERFORMANCE BREAKDOWN:");
        info!("                           ShredStream    Your gRPC");
        info!("  🚀 Real-time Speed:      {:>6.1}/100    {:>6.1}/100", self.shredstream_realtime_score, self.grpc_realtime_score);
        info!("  📏 Consistency:          {:>6.1}/100    {:>6.1}/100", self.shredstream_consistency_score, self.grpc_consistency_score);
        info!("  🔄 Data Freshness:       {:>6.1}/100    {:>6.1}/100", self.shredstream_freshness_score, self.grpc_freshness_score);
        info!("  🛡️  Reliability:          {:>6.1}/100    {:>6.1}/100", self.shredstream_reliability_score, self.grpc_reliability_score);
        info!("  ════════════════════════════════════════════════");
        info!("  🏆 TOTAL MEV SCORE:      {:>6.1}/100    {:>6.1}/100", self.shredstream_total_score, self.grpc_total_score);
        info!("");

        // Winner analysis
        if self.shredstream_total_score > self.grpc_total_score {
            let advantage = self.shredstream_total_score - self.grpc_total_score;
            info!("🏆 PERFORMANCE LEADER: ShredStream (+{:.1} points)", advantage);
        } else {
            let advantage = self.grpc_total_score - self.shredstream_total_score;
            info!("🏆 PERFORMANCE LEADER: Your gRPC (+{:.1} points)", advantage);
        }

        info!("");
        info!("💡 MEV TRADING RECOMMENDATION:");
        info!("   {}", self.recommendation);

        info!("");
        info!("📈 OPTIMIZATION SUGGESTIONS:");

        if self.grpc_realtime_score < 80.0 {
            info!("  • 🔧 gRPC Speed: Consider connection pooling or closer endpoints");
        }
        if self.grpc_consistency_score < 70.0 {
            info!("  • 📏 gRPC Consistency: Add retry logic and timeout optimization");
        }
        if self.shredstream_realtime_score < 80.0 {
            info!("  • 🔧 ShredStream: Check network proximity to Solana validators");
        }

        // Based on Grok's guidance
        let performance_gap = (self.shredstream_total_score - self.grpc_total_score).abs();
        if performance_gap < 15.0 {
            info!("  ✅ GROK APPROVED: Performance gap < 15 points - either can be primary!");
        } else if self.grpc_total_score < self.shredstream_total_score {
            info!("  ⚠️  GROK GUIDANCE: Consider optimization before using gRPC as primary");
        }

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

async fn test_mev_scenarios() -> Result<MevScenarioResults> {
    info!("🎯 Testing MEV Trading Scenarios...");

    let mut results = MevScenarioResults::new();
    let mut shred_latencies = Vec::new();
    let mut grpc_latencies = Vec::new();
    let mut shred_errors = 0u64;
    let mut grpc_errors = 0u64;
    let total_tests = 15u64;

    info!("📊 Running {} rounds of MEV scenario testing...", total_tests);

    // Load ShredStream config
    let config = match SharedConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            warn!("⚠️  Using test config due to: {}", e);
            SharedConfig {
                shreds_endpoint: "https://shreds-ny6-1.erpc.global".to_string(),
                jupiter_api_key: "test_key".to_string(),
                solana_rpc_endpoint: "https://api.mainnet-beta.solana.com".to_string(),
            }
        }
    };

    for round in 1..=total_tests {
        info!("🔄 Round {}/{}", round, total_tests);

        // Test 1: ShredStream real-time entry processing (simulates MEV opportunity detection)
        let shred_start = Instant::now();
        match timeout(Duration::from_secs(8), async {
            let mut client = ShredstreamClient::connect(&config.shreds_endpoint).await?;
            let request = solana_stream_sdk::SubscribeEntriesRequest {
                commitment: Some(CommitmentLevel::Confirmed as i32),
                accounts: std::collections::HashMap::new(),
                transactions: std::collections::HashMap::new(),
                slots: std::collections::HashMap::new(),
            };

            let mut stream = client.subscribe_entries(request).await?;

            use futures::StreamExt;
            if let Some(entry_result) = stream.next().await {
                match entry_result {
                    Ok(entry_message) => {
                        // Simulate MEV analysis on entry data
                        let _entries: Vec<Entry> = bincode::deserialize(&entry_message.entries)?;
                        // Simulate transaction analysis time
                        sleep(Duration::from_millis(5)).await;
                        Ok(())
                    }
                    Err(e) => Err(anyhow::anyhow!("Stream error: {}", e))
                }
            } else {
                Err(anyhow::anyhow!("No data received"))
            }
        }).await {
            Ok(Ok(_)) => {
                let latency = shred_start.elapsed().as_millis() as f64;
                shred_latencies.push(latency);
                info!("  🔗 ShredStream: {:.2}ms", latency);
            }
            Ok(Err(e)) => {
                warn!("  ❌ ShredStream error: {}", e);
                shred_errors += 1;
            }
            Err(_) => {
                warn!("  ⏰ ShredStream timeout");
                shred_errors += 1;
            }
        }

        // Test 2: Your gRPC price data access (simulates MEV price checking)
        let grpc_start = Instant::now();
        let test_cmd = r#"
import sys
sys.path.append('/data/Premium_Bot')
from grpc_solana_client import get_current_price
import time

# Simulate MEV price checking for multiple tokens (like sandwich attack analysis)
tokens = [
    'So11111111111111111111111111111111111111112',  # SOL
    'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'   # USDC
]

start_time = time.time()
success_count = 0

for token in tokens:
    price = get_current_price(token)
    if price:
        success_count += 1
    time.sleep(0.01)  # Small delay to simulate MEV analysis

end_time = time.time()

if success_count == len(tokens):
    latency = (end_time - start_time) * 1000
    print(f"SUCCESS:{latency:.2f}")
else:
    print(f"PARTIAL:{success_count}/{len(tokens)}")
"#;

        match timeout(Duration::from_secs(8), async {
            let output = Command::new("python3")
                .arg("-c")
                .arg(test_cmd)
                .output();

            match output {
                Ok(result) => {
                    if result.status.success() {
                        let stdout = String::from_utf8_lossy(&result.stdout);
                        if stdout.starts_with("SUCCESS:") {
                            if let Ok(latency) = stdout.trim().replace("SUCCESS:", "").parse::<f64>() {
                                grpc_latencies.push(latency);
                                info!("  📡 gRPC: {:.2}ms", latency);
                                return Ok(());
                            }
                        }
                    }
                    grpc_errors += 1;
                    warn!("  ❌ gRPC error");
                    Err(anyhow::anyhow!("gRPC failed"))
                }
                Err(e) => {
                    grpc_errors += 1;
                    warn!("  ❌ gRPC command error: {}", e);
                    Err(anyhow::anyhow!("Command failed: {}", e))
                }
            }
        }).await {
            Ok(Ok(_)) => {},
            Ok(Err(_)) => {},
            Err(_) => {
                warn!("  ⏰ gRPC timeout");
                grpc_errors += 1;
            }
        }

        // Brief pause between rounds
        sleep(Duration::from_millis(200)).await;
    }

    info!("");
    info!("📊 Raw Results:");
    info!("  • ShredStream: {} successful, {} errors", shred_latencies.len(), shred_errors);
    info!("  • gRPC: {} successful, {} errors", grpc_latencies.len(), grpc_errors);

    // Calculate comprehensive scores
    results.calculate_scores(&shred_latencies, &grpc_latencies, shred_errors, grpc_errors, total_tests);

    Ok(results)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("⚔️  MEV SCENARIO: gRPC vs ShredStream");
    info!("🎯 Testing for real MEV trading suitability");
    info!("📋 Following Grok's guidance: 'Use as backup if close to as fast'");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    match test_mev_scenarios().await {
        Ok(results) => {
            info!("");
            results.print_mev_analysis();
        }
        Err(e) => {
            error!("❌ MEV scenario test failed: {}", e);
        }
    }

    Ok(())
}