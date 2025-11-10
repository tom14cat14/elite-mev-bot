use anyhow::Result;
use bytes::BytesMut;
use futures::StreamExt;
use solana_entry::entry::Entry;
use solana_stream_sdk::ShredstreamClient;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};
use crate::mev_sandwich_detector::{detect_sandwich_opportunities, SandwichConfig};

pub struct ShredStreamProcessor {
    pub endpoint: String,
    pub buffer: BytesMut,
    current_slot: Arc<RwLock<u64>>,
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
            current_slot: Arc::new(RwLock::new(0)),
        }
    }

    /// Process ShredStream data - connects ONCE and keeps stream alive (ORE bot pattern)
    pub async fn process_real_shreds(&mut self) -> Result<ShredStreamEvent> {
        let start = Instant::now();

        info!("🔌 Connecting to ShredStream (ORE bot pattern - persistent connection)");
        info!("📡 Endpoint: {}", self.endpoint);

        // Connect ONCE to ShredStream with 10-second timeout (fail fast, retry in main loop)
        let mut client = match tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            ShredstreamClient::connect(&self.endpoint)
        ).await {
            Ok(Ok(client)) => {
                info!("✅ ShredStream connected successfully in {:.2}ms", start.elapsed().as_millis());
                client
            }
            Ok(Err(e)) => {
                error!("❌ ShredStream connection failed: {}", e);
                return Err(anyhow::anyhow!("ShredStream connection failed: {}", e));
            }
            Err(_) => {
                error!("❌ ShredStream connection TIMEOUT after 10 seconds");
                return Err(anyhow::anyhow!("ShredStream connection timeout - will retry"));
            }
        };

        let request = ShredstreamClient::create_empty_entries_request();
        let mut stream = client.subscribe_entries(request).await?;

        info!("✅ Subscribed to ShredStream entries - starting continuous processing loop");

        let mut total_entries = 0u64;

        // CRITICAL: Keep stream alive in loop (like ORE bot does)
        while let Some(slot_entry_result) = stream.next().await {
            match slot_entry_result {
                Ok(slot_entry) => {
                    let slot = slot_entry.slot;

                    // Update current slot
                    {
                        let mut current = self.current_slot.write().await;
                        *current = slot;
                    }

                    // Deserialize entries from binary data
                    match bincode::deserialize::<Vec<Entry>>(&slot_entry.entries) {
                        Ok(entries) => {
                            total_entries += entries.len() as u64;

                            let mut opportunities = 0u64;
                            let mut total_bytes = 0usize;

                            // MEV SANDWICH DETECTION - Detect victim swaps
                            let sandwich_config = SandwichConfig::default();
                            let sandwich_opps = detect_sandwich_opportunities(&entries, &sandwich_config);

                            if !sandwich_opps.is_empty() {
                                info!("🎯 {} sandwich opportunities detected in slot {} | Total entries: {}",
                                      sandwich_opps.len(), slot, total_entries);
                                opportunities += sandwich_opps.len() as u64;

                                // Return immediately when opportunities found
                                let latency_us = start.elapsed().as_micros() as f64;
                                return Ok(ShredStreamEvent {
                                    opportunity_count: opportunities,
                                    latency_us,
                                    data_size_bytes: slot_entry.entries.len(),
                                    sandwich_opportunities: sandwich_opps,
                                });
                            }

                            // Process entries for PumpFun opportunities
                            for entry in &entries {
                                for tx in &entry.transactions {
                                    // Store transaction data in buffer
                                    if let Ok(serialized) = bincode::serialize(tx) {
                                        total_bytes += serialized.len();
                                        self.buffer.clear();
                                        self.buffer.extend_from_slice(&serialized);

                                        // Filter for PumpFun opportunities
                                        if let Ok(count) = self.filter_pumpfun_shreds(&self.buffer) {
                                            if count > 0 {
                                                opportunities += count;
                                            }
                                        }
                                    }
                                }
                            }

                            // Return event even if no opportunities (keeps main loop alive)
                            if opportunities > 0 || total_entries % 100 == 0 {
                                let latency_us = start.elapsed().as_micros() as f64;

                                if total_entries % 100 == 0 {
                                    debug!("📊 ShredStream: {} entries processed, slot {}", total_entries, slot);
                                }

                                return Ok(ShredStreamEvent {
                                    opportunity_count: opportunities,
                                    latency_us,
                                    data_size_bytes: total_bytes,
                                    sandwich_opportunities: Vec::new(),
                                });
                            }
                        }
                        Err(e) => {
                            warn!("⚠️ Failed to deserialize entries in slot {}: {}", slot, e);
                            // Continue processing next entry instead of failing
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!("⚠️ ShredStream error: {} - will reconnect", e);
                    // Return error to trigger reconnection in main loop
                    return Err(anyhow::anyhow!("ShredStream stream error: {}", e));
                }
            }
        }

        warn!("🛑 ShredStream stream ended - returning to trigger reconnect");
        Err(anyhow::anyhow!("ShredStream stream ended"))
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
