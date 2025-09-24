use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🧪 ShredStream Data Flow Test Starting...");

    // Test 1: Check if MEV bot binary exists
    info!("🔍 Checking MEV bot binary...");
    let production_bin = std::path::Path::new("target/debug/elite_mev_bot_v2_1_production");
    let release_bin = std::path::Path::new("target/release/elite_mev_bot_v2_1_production");

    if production_bin.exists() {
        info!("✅ Found debug binary: {:?}", production_bin);
    } else if release_bin.exists() {
        info!("✅ Found release binary: {:?}", release_bin);
    } else {
        warn!("❌ No MEV bot binary found - building first");
        info!("🔧 Building MEV bot...");

        let output = tokio::process::Command::new("cargo")
            .args(&["build", "--bin", "elite_mev_bot_v2_1_production"])
            .output()
            .await?;

        if output.status.success() {
            info!("✅ MEV bot built successfully");
        } else {
            error!("❌ Failed to build MEV bot: {}", String::from_utf8_lossy(&output.stderr));
            return Ok(());
        }
    }

    // Test 2: Run MEV bot for 10 seconds to capture ShredStream data
    info!("🚀 Starting MEV bot for 10-second data capture...");

    let start = Instant::now();
    let mut child = tokio::process::Command::new("cargo")
        .args(&["run", "--bin", "elite_mev_bot_v2_1_production"])
        .env("RUST_LOG", "info")
        .env("PAPER_TRADING", "true")
        .env("ENABLE_REAL_TRADING", "false")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // Wait for 10 seconds
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Kill the process
    child.kill().await.ok();
    let output = child.wait_with_output().await?;

    let elapsed = start.elapsed();
    info!("⏱️  MEV bot ran for {:?}", elapsed);

    // Test 3: Analyze output for ShredStream data
    info!("📊 Analyzing MEV bot output for ShredStream data...");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut shredstream_messages = 0;
    let mut processing_cycles = 0;
    let mut opportunities_detected = 0;
    let mut connection_established = false;

    for line in stdout.lines().chain(stderr.lines()) {
        if line.contains("ShredStream") || line.contains("shredstream") {
            shredstream_messages += 1;
            info!("📡 ShredStream: {}", line);
        }

        if line.contains("Processing") || line.contains("processing") {
            processing_cycles += 1;
        }

        if line.contains("opportunity") || line.contains("Opportunity") {
            opportunities_detected += 1;
            info!("🎯 Opportunity: {}", line);
        }

        if line.contains("Connected") || line.contains("connected") {
            connection_established = true;
            info!("🔌 Connection: {}", line);
        }

        if line.contains("ERROR") || line.contains("error") {
            error!("❌ Error: {}", line);
        }
    }

    // Test 4: Results summary
    info!("");
    info!("📋 ShredStream Data Test Results:");
    info!("   🔌 Connection established: {}", connection_established);
    info!("   📡 ShredStream messages: {}", shredstream_messages);
    info!("   🔄 Processing cycles: {}", processing_cycles);
    info!("   🎯 Opportunities detected: {}", opportunities_detected);
    info!("   ⏱️  Runtime: {:?}", elapsed);

    if shredstream_messages > 0 {
        info!("✅ SUCCESS: ShredStream data is flowing through the system");
    } else {
        warn!("⚠️  NO SHREDSTREAM DATA: Check network connectivity and configuration");
    }

    if processing_cycles > 0 {
        info!("✅ SUCCESS: Processing cycles are running");
    } else {
        warn!("⚠️  NO PROCESSING: Check bot implementation");
    }

    if opportunities_detected > 0 {
        info!("✅ SUCCESS: Opportunities are being detected");
    } else {
        info!("ℹ️  No opportunities detected (normal for short test duration)");
    }

    info!("");
    info!("🏁 ShredStream data flow test completed!");

    Ok(())
}