use anyhow::Result;
use bytes::BytesMut;
use futures::StreamExt;
use solana_entry::entry::Entry;
use solana_stream_sdk::ShredstreamClient;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use crate::mev_sandwich_detector::{detect_sandwich_opportunities, SandwichConfig};

pub struct ShredStreamProcessor {
    pub endpoint: String,
    pub buffer: BytesMut,
    stream_data: Arc<RwLock<Option<Vec<Entry>>>>,  // Shared buffer for entries
    initialized: bool,
}

#[derive(Debug, Clone)]
pub struct ShredStreamEvent {
    pub opportunity_count: u64,
    pub latency_us: f64,
    pub data_size_bytes: usize,
    pub sandwich_opportunities: Vec<crate::mev_sandwich_detector::SandwichOpportunity>,
}

impl ShredStreamProcessor {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            buffer: BytesMut::with_capacity(65535),
            stream_data: Arc::new(RwLock::new(None)),
            initialized: false,
        }
    }

    /// Initialize persistent gRPC-over-HTTPS connection and start background streaming
    pub async fn initialize(&mut self) -> Result<()> {
        println!("🔥🔥🔥 SHREDSTREAM_PROCESSOR: initialize() ENTRY POINT");
        eprintln!("🔥🔥🔥 SHREDSTREAM_PROCESSOR: initialize() ENTRY POINT");
        info!("🔥🔥🔥 DEBUG: Entered initialize() function");
        info!("🔥 DEBUG: self.initialized = {}", self.initialized);

        if self.initialized {
            println!("⏭️  EARLY RETURN: Already initialized");
            eprintln!("⏭️  EARLY RETURN: Already initialized");
            info!("⏭️  EARLY RETURN: Already initialized");
            return Ok(());
        }

        println!("✅ CHECKPOINT 1: Passed self.initialized check");
        eprintln!("✅ CHECKPOINT 1: Passed self.initialized check");
        info!("✅ CHECKPOINT 1: Passed self.initialized check");
        info!("🔌 Initializing gRPC ShredStream connection: {}", self.endpoint);

        // Connect to ShredStream via gRPC-over-HTTPS
        println!("✅ CHECKPOINT 2: About to connect to ShredStream at: {}", self.endpoint);
        eprintln!("✅ CHECKPOINT 2: About to connect to ShredStream at: {}", self.endpoint);
        info!("✅ CHECKPOINT 2: About to connect to ShredStream at: {}", self.endpoint);

        let mut client = ShredstreamClient::connect(&self.endpoint).await
            .map_err(|e| anyhow::anyhow!("ShredStream gRPC connection failed (check IP whitelist): {}", e))?;

        println!("✅ CHECKPOINT 3: ShredStream client connection successful");
        eprintln!("✅ CHECKPOINT 3: ShredStream client connection successful");
        info!("✅ CHECKPOINT 3: ShredStream client connection successful");
        info!("✅ Persistent ShredStream gRPC connection established");

        // 🔧 CRITICAL FIX: Account-based filtering is unreliable with ERPC ShredStream
        // Instead, subscribe to ALL entries and filter locally in detect_sandwich_opportunities()
        // This is the same approach used by the working ORE bot implementation
        println!("✅ CHECKPOINT 4: Creating unfiltered subscription (ERPC ShredStream fix)");
        eprintln!("✅ CHECKPOINT 4: Creating unfiltered subscription (ERPC ShredStream fix)");
        info!("📡 Subscribing to ALL ShredStream entries (local DEX filtering)");
        info!("🔍 Reason: Account-based filtering unreliable with ERPC ShredStream");

        // Subscribe to ALL entries (no account filtering)
        println!("✅ CHECKPOINT 5: Creating subscription request");
        eprintln!("✅ CHECKPOINT 5: Creating subscription request");
        info!("🔍 DEBUG: Creating unfiltered subscription request");
        let request = ShredstreamClient::create_empty_entries_request();

        println!("✅ CHECKPOINT 6: About to subscribe to entries stream");
        eprintln!("✅ CHECKPOINT 6: About to subscribe to entries stream");
        info!("🔍 DEBUG: Subscribing to entries stream");
        let mut stream = client.subscribe_entries(request).await?;
        println!("✅ CHECKPOINT 7: Subscription successful!");
        eprintln!("✅ CHECKPOINT 7: Subscription successful!");
        info!("🔍 DEBUG: Subscription successful");
        info!("📡 Subscribed to ShredStream for ALL entries (local DEX filtering)");

        // Start background task to continuously stream data
        println!("✅ CHECKPOINT 8: About to spawn background task");
        eprintln!("✅ CHECKPOINT 8: About to spawn background task");
        info!("🔍 DEBUG: About to spawn background task");
        let stream_data = self.stream_data.clone();
        tokio::spawn(async move {
            let mut entries_processed = 0u64;

            println!("✅ CHECKPOINT 9: Background task started!");
            eprintln!("✅ CHECKPOINT 9: Background task started!");
            info!("🚀 Background ShredStream processor started");
            info!("🔍 DEBUG: Background task is now running");

            let mut loop_count = 0u64;
            while let Some(slot_entry_result) = stream.next().await {
                loop_count += 1;
                if loop_count % 100 == 0 {
                    println!("📦 Background task loop iteration: {}", loop_count);
                    eprintln!("📦 Background task loop iteration: {}", loop_count);
                    info!("📦 Background task loop iteration: {}", loop_count);
                }

                match slot_entry_result {
                    Ok(slot_entry) => {
                        println!("✅ Received slot entry from ShredStream (entries_processed: {})", entries_processed);
                        eprintln!("✅ Received slot entry from ShredStream (entries_processed: {})", entries_processed);
                        info!("✅ Received slot entry from ShredStream (entries_processed: {})", entries_processed);

                        // Deserialize entries from binary data
                        match bincode::deserialize::<Vec<Entry>>(&slot_entry.entries) {
                            Ok(entries) => {
                                entries_processed += entries.len() as u64;
                                println!("✅ Deserialized {} entries (total processed: {})", entries.len(), entries_processed);
                                eprintln!("✅ Deserialized {} entries (total processed: {})", entries.len(), entries_processed);
                                info!("✅ Deserialized {} entries (total processed: {})", entries.len(), entries_processed);

                                // Update shared buffer with latest entries
                                {
                                    let mut data = stream_data.write().await;
                                    *data = Some(entries);
                                }

                                if entries_processed % 100 == 0 {
                                    debug!("📦 Processed {} entries from ShredStream", entries_processed);
                                }
                            }
                            Err(e) => {
                                println!("⚠️ Failed to deserialize entries: {}", e);
                                eprintln!("⚠️ Failed to deserialize entries: {}", e);
                                warn!("⚠️ Failed to deserialize entries: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️ ShredStream error: {}", e);
                        eprintln!("⚠️ ShredStream error: {}", e);
                        warn!("⚠️ ShredStream error: {}", e);
                    }
                }
            }

            warn!("🛑 ShredStream background processor ended");
        });

        println!("✅ CHECKPOINT 10: Background task spawned successfully");
        eprintln!("✅ CHECKPOINT 10: Background task spawned successfully");
        info!("🔍 DEBUG: Background task spawned successfully");
        self.initialized = true;
        println!("✅ CHECKPOINT 11: self.initialized set to true");
        eprintln!("✅ CHECKPOINT 11: self.initialized set to true");
        info!("🔍 DEBUG: self.initialized set to true");
        println!("✅✅✅ CHECKPOINT 12: initialize() COMPLETED SUCCESSFULLY!");
        eprintln!("✅✅✅ CHECKPOINT 12: initialize() COMPLETED SUCCESSFULLY!");
        info!("✅✅✅ CHECKPOINT 12: initialize() COMPLETED SUCCESSFULLY!");
        Ok(())
    }

    /// Ultra-fast shred processing using persistent gRPC connection
    /// This reads from the shared buffer populated by the background task
    pub async fn process_real_shreds(&mut self) -> Result<ShredStreamEvent> {
        let start = Instant::now();

        // Initialize if not already done
        if !self.initialized {
            self.initialize().await?;
            // Give it a moment to start receiving data
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Read latest entries from shared buffer
        let entries_opt = {
            let mut data = self.stream_data.write().await;
            data.take()  // Take ownership and clear buffer
        };

        if let Some(entries) = entries_opt {
            let mut opportunities = 0u64;
            let mut total_bytes = 0usize;

            // MEV SANDWICH DETECTION - Detect victim swaps
            let sandwich_config = SandwichConfig::default();
            let sandwich_opps = detect_sandwich_opportunities(&entries, &sandwich_config);

            if !sandwich_opps.is_empty() {
                info!("🎯 SANDWICH OPPORTUNITIES DETECTED: {}", sandwich_opps.len());
                for opp in &sandwich_opps {
                    info!("  💰 {} swap: {:.4} SOL on {} (sig: {})",
                          opp.dex_name, opp.estimated_sol_value, opp.dex_name, &opp.signature[..20]);
                }
                opportunities += sandwich_opps.len() as u64;
            }

            // Process entries for opportunities
            for entry in &entries {
                for tx in &entry.transactions {
                    // Store transaction data in buffer for opportunity detection
                    if let Ok(serialized) = bincode::serialize(tx) {
                        total_bytes += serialized.len();
                        self.buffer.clear();
                        self.buffer.extend_from_slice(&serialized);

                        // Filter for PumpFun opportunities
                        if let Ok(count) = self.filter_pumpfun_shreds(&self.buffer) {
                            opportunities += count;
                        }
                    }
                }
            }

            let latency_us = start.elapsed().as_micros() as f64;

            Ok(ShredStreamEvent {
                opportunity_count: opportunities,
                latency_us,
                data_size_bytes: total_bytes,
                sandwich_opportunities: sandwich_opps,
            })
        } else {
            // No new data available, return immediately with zero count
            Ok(ShredStreamEvent {
                opportunity_count: 0,
                latency_us: start.elapsed().as_micros() as f64,
                data_size_bytes: 0,
                sandwich_opportunities: Vec::new(),
            })
        }
    }

    /// Get the latest raw ShredStream data for processing
    pub fn get_latest_data(&self) -> Vec<u8> {
        if self.buffer.is_empty() {
            vec![]
        } else {
            self.buffer.to_vec()
        }
    }

    fn filter_pumpfun_shreds(&self, data: &BytesMut) -> Result<u64> {
        // Simplified shred filtering for PumpFun program ID
        let _pumpfun_program = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

        // In a real implementation, this would:
        // 1. Parse transactions from shreds
        // 2. Filter for PumpFun program calls
        // 3. Detect new token creations
        // 4. Extract token metadata

        // For now, detect opportunities based on transaction structure
        // PumpFun transactions typically have specific instruction patterns
        if data.len() > 200 {
            // Check for PumpFun program ID in transaction data
            let pumpfun_bytes = bs58::decode(_pumpfun_program).into_vec().ok();
            if let Some(program_id) = pumpfun_bytes {
                // Simple pattern matching - look for program ID in tx data
                if data.windows(program_id.len()).any(|window| window == program_id.as_slice()) {
                    return Ok(1); // Found PumpFun transaction!
                }
            }
        }

        Ok(0) // No opportunities
    }
}
