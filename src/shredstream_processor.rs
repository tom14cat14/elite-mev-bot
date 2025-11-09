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
    stream: Option<tonic::Streaming<solana_stream_sdk::shredstream_proto::Entry>>,
    current_slot: Arc<RwLock<u64>>,
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
            stream: None,
            current_slot: Arc::new(RwLock::new(0)),
            initialized: false,
        }
    }

    /// Initialize persistent gRPC-over-HTTPS connection
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        info!("🔌 Initializing ShredStream for MEV sandwich detection");
        info!("📡 Endpoint: {}", self.endpoint);

        // Connect to ShredStream via gRPC-over-HTTPS
        let mut client = ShredstreamClient::connect(&self.endpoint).await
            .map_err(|e| anyhow::anyhow!("ShredStream connection failed: {}", e))?;

        info!("✅ ShredStream connection established");

        // Subscribe to ALL entries (no filtering - per ORE bot working implementation)
        let request = ShredstreamClient::create_empty_entries_request();
        let stream = client.subscribe_entries(request).await?;

        info!("✅ Subscribed to ShredStream (will filter DEX swaps locally)");

        // Store stream for direct consumption in main loop
        self.stream = Some(stream);
        self.initialized = true;

        Ok(())
    }

    /// Process ShredStream data directly in main loop (NO SPAWN)
    /// This reads from the stream WITHOUT spawning a background task
    pub async fn process_real_shreds(&mut self) -> Result<ShredStreamEvent> {
        let start = Instant::now();

        // Initialize if not already done
        if !self.initialized {
            self.initialize().await?;
        }

        // Get stream reference
        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow::anyhow!("ShredStream not initialized"))?;

        // CRITICAL: Call stream.next() directly in main task (NO SPAWN)
        match stream.next().await {
            Some(slot_entry_result) => {
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
                                let mut opportunities = 0u64;
                                let mut total_bytes = 0usize;

                                // MEV SANDWICH DETECTION - Detect victim swaps
                                let sandwich_config = SandwichConfig::default();
                                let sandwich_opps = detect_sandwich_opportunities(&entries, &sandwich_config);

                                if !sandwich_opps.is_empty() {
                                    debug!("🎯 {} sandwich opportunities detected in slot {}",
                                           sandwich_opps.len(), slot);
                                    opportunities += sandwich_opps.len() as u64;
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
                            }
                            Err(e) => {
                                warn!("⚠️ Failed to deserialize entries: {}", e);
                                Ok(ShredStreamEvent {
                                    opportunity_count: 0,
                                    latency_us: start.elapsed().as_micros() as f64,
                                    data_size_bytes: 0,
                                    sandwich_opportunities: Vec::new(),
                                })
                            }
                        }
                    }
                    Err(e) => {
                        Err(anyhow::anyhow!("ShredStream error: {}", e))
                    }
                }
            }
            None => {
                Err(anyhow::anyhow!("ShredStream ended: stream returned None"))
            }
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
