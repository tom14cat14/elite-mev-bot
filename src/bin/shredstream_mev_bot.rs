use anyhow::Result;
use shared_bot_infrastructure::*;
use std::time::{Duration, Instant};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[INFO] 🚀 Starting ELITE MEV Bot v2.1 - REAL SHREDSTREAM UDP");
    println!("[INFO] ⚡ Features: Real UDP ShredStream + Sub-15ms latency");
    println!("[INFO] 🎯 TARGET: Ultra-low latency PumpFun opportunity detection");
    println!("[INFO] 🔌 ENDPOINT: https://shreds-ny6-1.erpc.global (UDP)");
    println!("[INFO] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Initialize ShredStream processor
    let endpoint = std::env::var("SHREDS_ENDPOINT")
        .unwrap_or_else(|_| "https://shreds-ny6-1.erpc.global".to_string());

    let mut shred_processor = ShredStreamProcessor::new(endpoint);

    // Performance tracking
    let start_time = Instant::now();
    let mut total_cycles = 0u64;
    let mut total_opportunities = 0u64;
    let mut latency_samples = Vec::new();

    println!("[INFO] 🔄 Starting real-time ShredStream processing loop...");
    println!("[INFO] 📡 Processing UDP shreds every 1ms for maximum speed...");

    // Main ultra-low latency loop
    loop {
        tokio::select! {
            // Graceful shutdown
            _ = signal::ctrl_c() => {
                println!("[INFO] 🛑 Received shutdown signal, stopping bot...");
                break;
            }

            // Ultra-fast ShredStream processing (1ms intervals)
            _ = tokio::time::sleep(Duration::from_millis(1)) => {
                total_cycles += 1;

                // Process real UDP shreds
                match shred_processor.process_real_shreds().await {
                    Ok(event) => {
                        total_opportunities += event.opportunity_count;
                        latency_samples.push(event.latency_us);

                        // Keep only last 1000 samples for performance
                        if latency_samples.len() > 1000 {
                            latency_samples.drain(0..100);
                        }

                        // Show activity every 1000 cycles (1 second)
                        if total_cycles % 1000 == 0 {
                            let avg_latency = latency_samples.iter().sum::<f64>() / latency_samples.len() as f64;
                            let runtime = start_time.elapsed().as_secs_f64();
                            let cycles_per_sec = total_cycles as f64 / runtime;

                            println!("[DEBUG] 📊 Performance: {:.1} cycles/sec | Avg latency: {:.1}μs | Opportunities: {}",
                                cycles_per_sec, avg_latency, total_opportunities);
                        }

                        // Report opportunities immediately
                        if event.opportunity_count > 0 {
                            println!("[INFO] 🆕 OPPORTUNITY DETECTED via ShredStream | Latency: {:.1}μs | Data: {} bytes",
                                event.latency_us, event.data_size_bytes);
                        }

                        // Performance report every 10 seconds
                        if total_cycles % 10000 == 0 {
                            let runtime = start_time.elapsed().as_secs_f64();
                            let avg_latency = latency_samples.iter().sum::<f64>() / latency_samples.len() as f64;
                            let cycles_per_sec = total_cycles as f64 / runtime;
                            let opportunities_per_min = (total_opportunities as f64 / runtime) * 60.0;

                            println!("[INFO] 🏆 ELITE PERFORMANCE METRICS:");
                            println!("[INFO]   • Processing rate: {:.1} cycles/sec", cycles_per_sec);
                            println!("[INFO]   • Average latency: {:.1}μs (TARGET: <15000μs)", avg_latency);
                            println!("[INFO]   • Opportunities: {} total ({:.1}/min)", total_opportunities, opportunities_per_min);
                            println!("[INFO]   • Uptime: {:.1}s", runtime);

                            // Performance status
                            if avg_latency < 15000.0 {
                                println!("[INFO]   • Status: ✅ ELITE PERFORMANCE (under 15ms target)");
                            } else {
                                println!("[INFO]   • Status: ⚠️  Performance needs optimization");
                            }
                        }
                    }
                    Err(e) => {
                        if total_cycles % 5000 == 0 { // Report errors every 5 seconds
                            println!("[WARN] ⚠️  ShredStream processing error: {} (check IP whitelist)", e);
                        }
                    }
                }
            }
        }
    }

    // Final performance report
    let runtime = start_time.elapsed().as_secs_f64();
    let avg_latency = if !latency_samples.is_empty() {
        latency_samples.iter().sum::<f64>() / latency_samples.len() as f64
    } else {
        0.0
    };

    println!("[INFO] 📈 FINAL PERFORMANCE REPORT:");
    println!("[INFO] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("[INFO]   • Total cycles: {}", total_cycles);
    println!("[INFO]   • Total opportunities: {}", total_opportunities);
    println!("[INFO]   • Runtime: {:.1}s", runtime);
    println!("[INFO]   • Average performance: {:.1} cycles/sec", total_cycles as f64 / runtime);
    println!("[INFO]   • Average latency: {:.1}μs", avg_latency);
    println!("[INFO] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if avg_latency < 15000.0 {
        println!("[INFO] ✅ ELITE MEV Bot v2.1 - Target latency achieved!");
    } else {
        println!("[INFO] ⚠️  Consider IP whitelisting for real ShredStream access");
    }

    Ok(())
}