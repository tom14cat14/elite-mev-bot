use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{info, warn};
use reqwest;
use serde_json::Value;

// Import our existing ShredStream infrastructure
use shared_bot_infrastructure::shredstream_processor::ShredStreamProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🧪 Simple ShredStream Test Starting...");
    info!("📡 Testing connection to: https://shreds-ny6-1.erpc.global");

    // Test 1: HTTP RPC connectivity
    test_http_rpc().await?;

    // Test 2: UDP ShredStream using existing infrastructure
    test_shredstream_udp().await?;

    info!("🏁 All tests completed!");
    Ok(())
}

async fn test_http_rpc() -> Result<()> {
    info!("🔄 Testing HTTP RPC connectivity...");

    let client = reqwest::Client::new();
    let start = Instant::now();

    // Test with a simple getHealth request
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getHealth"
    });

    match timeout(Duration::from_secs(10),
        client.post("https://shreds-ny6-1.erpc.global")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
    ).await {
        Ok(Ok(response)) => {
            let latency = start.elapsed();
            info!("✅ HTTP RPC response in {:?}", latency);
            info!("📊 Status: {}", response.status());

            if let Ok(body) = response.text().await {
                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    info!("✅ Valid JSON response");
                    if let Some(result) = json.get("result") {
                        info!("🎯 Health: {:?}", result);
                    }
                }
            }
        }
        Ok(Err(e)) => {
            warn!("❌ HTTP RPC failed: {}", e);
        }
        Err(_) => {
            warn!("⏰ HTTP RPC timed out");
        }
    }

    // Test slot request
    let slot_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "getSlot",
        "params": []
    });

    let start = Instant::now();
    if let Ok(Ok(response)) = timeout(Duration::from_secs(5),
        client.post("https://shreds-ny6-1.erpc.global")
            .header("Content-Type", "application/json")
            .json(&slot_payload)
            .send()
    ).await {
        let latency = start.elapsed();
        info!("✅ Slot request in {:?}", latency);

        if let Ok(body) = response.text().await {
            if let Ok(json) = serde_json::from_str::<Value>(&body) {
                if let Some(result) = json.get("result") {
                    info!("📊 Current slot: {}", result);
                }
            }
        }
    }

    Ok(())
}

async fn test_shredstream_udp() -> Result<()> {
    info!("🔄 Testing UDP ShredStream using existing processor...");

    let mut processor = ShredStreamProcessor::new("https://shreds-ny6-1.erpc.global".to_string());

    // Test 5 iterations to see data flow
    for i in 1..=5 {
        info!("📡 Test iteration {}/5", i);

        let start = Instant::now();
        match processor.process_real_shreds().await {
            Ok(event) => {
                let total_latency = start.elapsed();
                info!("✅ Iteration {}: {} opportunities, {:.1}μs latency, {} bytes, total: {:?}",
                    i, event.opportunity_count, event.latency_us, event.data_size_bytes, total_latency);

                // Show data details if we got any
                if event.data_size_bytes > 0 {
                    let data = processor.get_latest_data();
                    info!("📦 Raw data preview: {} bytes, first 32 bytes: {:?}",
                        data.len(),
                        &data[..data.len().min(32)]);
                }
            }
            Err(e) => {
                warn!("❌ Iteration {} failed: {}", i, e);
            }
        }

        // Small delay between tests
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    info!("🎯 ShredStream UDP test completed");
    Ok(())
}