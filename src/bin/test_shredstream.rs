use anyhow::Result;
use futures::StreamExt;
use solana_entry::entry::Entry;
use solana_stream_sdk::ShredstreamClient;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== SHREDSTREAM MINIMAL TEST ===");
    println!("Endpoint: https://shreds-ny6-1.erpc.global");

    // Connect to ShredStream
    println!("\n1. Connecting to ShredStream...");
    let mut client = ShredstreamClient::connect("https://shreds-ny6-1.erpc.global").await?;
    println!("   ✅ Connected successfully");

    // Subscribe to all entries
    println!("\n2. Subscribing to entries...");
    let request = ShredstreamClient::create_empty_entries_request();
    let mut stream = client.subscribe_entries(request).await?;
    println!("   ✅ Subscribed successfully");

    // Try to receive first message with timeout
    println!("\n3. Waiting for first message (30s timeout)...");
    println!("   This will hang if ShredStream isn't sending data");

    match tokio::time::timeout(std::time::Duration::from_secs(30), stream.next()).await {
        Ok(Some(Ok(slot_entry))) => {
            println!("   ✅ RECEIVED DATA!");
            println!("   Slot: {}", slot_entry.slot);
            println!("   Entry bytes: {}", slot_entry.entries.len());

            // Try to deserialize
            match bincode::deserialize::<Vec<Entry>>(&slot_entry.entries) {
                Ok(entries) => {
                    let tx_count: usize = entries.iter().map(|e| e.transactions.len()).sum();
                    println!("   Entries: {}", entries.len());
                    println!("   Transactions: {}", tx_count);
                    println!("\n🎉 SUCCESS: ShredStream is working!");
                }
                Err(e) => {
                    println!("   ⚠️  Deserialization error: {}", e);
                }
            }
        }
        Ok(Some(Err(e))) => {
            println!("   ❌ Stream error: {}", e);
        }
        Ok(None) => {
            println!("   ❌ Stream ended unexpectedly");
        }
        Err(_) => {
            println!("   ❌ TIMEOUT: No data received in 30 seconds");
            println!("\nPossible causes:");
            println!("  1. IP not whitelisted for ShredStream");
            println!("  2. ShredStream service is down");
            println!("  3. Network connectivity issue");
            println!("  4. Firewall blocking gRPC traffic");
        }
    }

    Ok(())
}
