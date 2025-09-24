/// Test real ShredStream connection to verify data collection works
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[INFO] 🔗 Testing real ShredStream connection to: https://shreds-ny6-1.erpc.global");

    let start_time = Instant::now();
    let endpoint = "https://shreds-ny6-1.erpc.global";

    // Test HTTP connectivity first
    println!("[INFO] 📡 Testing HTTP connectivity...");

    let client = reqwest::Client::new();

    match tokio::time::timeout(
        Duration::from_secs(10),
        client.get(endpoint).send()
    ).await {
        Ok(Ok(response)) => {
            println!("[INFO] ✅ HTTP connection successful");
            println!("[INFO]   • Status: {}", response.status());
            println!("[INFO]   • Headers: {:?}", response.headers());

            // Try to get some response body
            match response.text().await {
                Ok(body) => {
                    let body_preview = if body.len() > 200 {
                        format!("{}... (truncated)", &body[..200])
                    } else {
                        body
                    };
                    println!("[INFO]   • Body preview: {}", body_preview);
                }
                Err(e) => println!("[WARN] ⚠️  Could not read response body: {}", e),
            }
        }
        Ok(Err(e)) => {
            println!("[ERROR] ❌ HTTP connection failed: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            println!("[ERROR] ❌ HTTP connection timeout after 10 seconds");
            return Err("Connection timeout".into());
        }
    }

    let connection_time = start_time.elapsed().as_millis();
    println!("[INFO] 🎯 Connection established in {}ms", connection_time);

    // Test if this is a WebSocket or different protocol endpoint
    println!("[INFO] 🔍 Testing WebSocket upgrade...");

    // Note: Real ShredStream might use WebSocket, gRPC, or custom protocol
    // This test shows what protocol the endpoint expects

    match tokio_tungstenite::connect_async(format!("wss://{}/ws", endpoint.trim_start_matches("https://"))).await {
        Ok((ws_stream, response)) => {
            println!("[INFO] ✅ WebSocket connection successful!");
            println!("[INFO]   • Response: {:?}", response);

            // Could test receiving data here
            println!("[INFO] 📊 WebSocket connected - ready for real-time data");
        }
        Err(e) => {
            println!("[WARN] ⚠️  WebSocket connection failed: {}", e);
            println!("[INFO] 💡 Endpoint might use different protocol (gRPC, HTTP streaming, etc.)");
        }
    }

    println!("[INFO] 🎉 ShredStream connectivity test completed");
    println!("[INFO] ⏱️  Total test time: {}ms", start_time.elapsed().as_millis());

    Ok(())
}