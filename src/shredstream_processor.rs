use anyhow::Result;
use bytes::BytesMut;
use futures::StreamExt;
use solana_entry::entry::Entry;
use solana_stream_sdk::{CommitmentLevel, ShredstreamClient};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

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
        if self.initialized {
            return Ok(());
        }

        info!("🔌 Initializing gRPC ShredStream connection: {}", self.endpoint);

        // Connect to ShredStream via gRPC-over-HTTPS
        let mut client = ShredstreamClient::connect(&self.endpoint).await
            .map_err(|e| anyhow::anyhow!("ShredStream gRPC connection failed (check IP whitelist): {}", e))?;

        info!("✅ Persistent ShredStream gRPC connection established");

        // Create subscription for ALL transactions (no filter for maximum speed)
        let request = ShredstreamClient::create_entries_request_for_accounts(
            vec![],                           // accounts (empty = all)
            vec![],                           // owner addresses (empty = all)
            vec![],                           // transaction accounts (empty = all)
            Some(CommitmentLevel::Processed), // commitment level
        );

        let mut stream = client.subscribe_entries(request).await?;
        info!("📡 Subscribed to ShredStream entries (all transactions)");

        // Start background task to continuously stream data
        let stream_data = self.stream_data.clone();
        tokio::spawn(async move {
            let mut entries_processed = 0u64;

            info!("🚀 Background ShredStream processor started");

            while let Some(slot_entry_result) = stream.next().await {
                match slot_entry_result {
                    Ok(slot_entry) => {
                        // Deserialize entries from binary data
                        match bincode::deserialize::<Vec<Entry>>(&slot_entry.entries) {
                            Ok(entries) => {
                                entries_processed += entries.len() as u64;

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
                                warn!("⚠️ Failed to deserialize entries: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ ShredStream error: {}", e);
                    }
                }
            }

            warn!("🛑 ShredStream background processor ended");
        });

        self.initialized = true;
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
            })
        } else {
            // No new data available, return immediately with zero count
            Ok(ShredStreamEvent {
                opportunity_count: 0,
                latency_us: start.elapsed().as_micros() as f64,
                data_size_bytes: 0,
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
