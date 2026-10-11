//! Leader Schedule Manager for 0slot Trading
//!
//! Fetches and caches Solana leader schedules to enable trading only on
//! geographically favorable leader slots. This is CRITICAL for 0slot success:
//! - US-East leaders: ~10-20ms latency to NY JITO endpoint = 0slot possible
//! - EU/Asia leaders: 100-200ms latency = victim confirms before bundle lands
//!
//! ⚡ 0SLOT: Only trade when current leader is "local" (low latency)

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Maximum latency (ms) to consider a leader "local" for 0slot trading
/// Leaders with higher latency = victim confirms before our bundle lands
const LOCAL_LEADER_LATENCY_THRESHOLD_MS: u64 = 50;

/// How often to refresh the leader schedule (in slots, ~400ms each)
const LEADER_SCHEDULE_REFRESH_SLOTS: u64 = 100;

/// How long to cache latency measurements (seconds)
const LATENCY_CACHE_DURATION_SECS: u64 = 3600; // 1 hour

/// Current slot tracker (updated by ShredStream)
static CURRENT_SLOT: AtomicU64 = AtomicU64::new(0);

/// Cached leader schedule: slot -> validator pubkey
static LEADER_SCHEDULE_CACHE: Lazy<RwLock<HashMap<u64, Pubkey>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Last slot when we fetched the leader schedule
static LAST_SCHEDULE_FETCH_SLOT: AtomicU64 = AtomicU64::new(0);

/// Cached validator latencies: pubkey -> (latency_ms, measured_at)
/// Uses DashMap for lock-free concurrent access
static VALIDATOR_LATENCY_CACHE: Lazy<DashMap<Pubkey, (u64, Instant)>> =
    Lazy::new(|| DashMap::new());

/// Known US-based validators (stake-weighted leaders with good latency to NY JITO)
/// EXPANDED LIST: Top validators by stake - most have US infrastructure
/// These represent ~60% of stake, significantly increasing local leader opportunities
/// Updated 2025-12-08 from live network data
static US_EAST_VALIDATORS: Lazy<Vec<Pubkey>> = Lazy::new(|| {
    vec![
        // ===== TOP VALIDATORS BY STAKE (likely US infrastructure) =====
        // #1 Helius - 13M SOL - US-based (San Francisco/Virginia)
        Pubkey::from_str("HEL1USMZKAL2odpNBj2oCjffnFGaYwmbGmyewGv1e2TU").unwrap(),
        // #2 High-stake validator - 12.5M SOL
        Pubkey::from_str("Fd7btgySsrjuo25CJCj7oE7VPMyezDhnx7pZkj2v69Nk").unwrap(),
        // #3 Jump/Jupiter - 12.3M SOL - US-based (Chicago)
        Pubkey::from_str("JupmVLmA8RoyTUbTMMuTtoPWHEiNQobxgTeGTrPNkzT").unwrap(),
        // #4 High-stake - 10.7M SOL
        Pubkey::from_str("DRpbCBMxVnDK7maPM5tGv6MvB3v1sRMC86PZ8okm21hy").unwrap(),
        // #5-10 High-stake validators - 6-8M SOL each
        Pubkey::from_str("q9XWcZ7T1wP4bW9SB4XgNNwjnFEJ982nE8aVbbNuwot").unwrap(),
        Pubkey::from_str("5pPRHniefFjkiaArbGX3Y8NUysJmQ9tMZg3FrFGwHzSm").unwrap(),
        Pubkey::from_str("DtdSSG8ZJRZVv5Jx7K1MeWp7Zxcu19GD5wQRGRpQ9uMF").unwrap(),
        Pubkey::from_str("JD549HsbJHeEKKUrKgg4Fj2iyv2RGjsV7NTZjZUrHybB").unwrap(),
        Pubkey::from_str("Awes4Tr6TX8JDzEhCZY2QVNimT6iD1zWHzf1vNyGvpLM").unwrap(),
        Pubkey::from_str("EvnRmnMrd69kFdbLMxWkTn1icZ7DCceRhvmb2SJXqDo4").unwrap(),
        // #11-15 Mid-high stake validators - 4-6M SOL each
        Pubkey::from_str("9jxgosAfHgHzwnxsHw4RAZYaLVokMbnYtmiZBreynGFP").unwrap(),
        Pubkey::from_str("CAo1dCGYrB6NhHh5xb1cGjUiu86iyCfMTENxgHumSve4").unwrap(),
        Pubkey::from_str("E1r4Psq84tHfQ6aPTvvDka4U3u8zPVD7gEUrH25RdxHL").unwrap(),
        Pubkey::from_str("BkoS26vBuaXnSowACdChi4WKid8UwmuPNhEJWa8KsLHd").unwrap(),
        Pubkey::from_str("9rkJMARqK6VBkcxGfKBAwnA44gPAfGxPbPsfsggFNDSQ").unwrap(),
        // #16-20 - 3.5-4.5M SOL each
        Pubkey::from_str("9UM8wQ8F5oMiRcP5YdqD6Lr4krpBWCD8LtgQYoisJd9i").unwrap(),
        // Chorus One - US infrastructure - 3.9M SOL
        Pubkey::from_str("ChorusmmK7i1AxXeiTtQgQZhQNiXYU84ULeaYF1EH15n").unwrap(),
        Pubkey::from_str("EkvdKhULbMFqjKBKotAzGi3kwMvMpYNDKJXXQQmi6C1f").unwrap(),
        Pubkey::from_str("5Cchr1XGEg7dbBXByV5NY2ad8jfxAM7HA3x8D56rq9Ux").unwrap(),
        Pubkey::from_str("J6etcxDdYjPHrtyvDXrbCkx3q9W1UjMj1vy1jBFPJEbK").unwrap(),
        // #21-25 - 3-3.5M SOL each
        Pubkey::from_str("BtsmiEEvnSuUnKxqXj2PZRYpPJAc7C34mGz8gtJ1DAaH").unwrap(),
        Pubkey::from_str("FBKFWadXZJahGtFitAsBvbqh5968gLY7dMBBJUoUjeNi").unwrap(),
        Pubkey::from_str("5EhGYUyQNrxgUbuYF4vbL2SZDT6RMfhq3yjeyevvULeC").unwrap(),
        Pubkey::from_str("GoeW4aFK4dGoekJySgUynWDxBZiQJqm8GDAF4H53tDK9").unwrap(),
        Pubkey::from_str("Hz5aLvpKScNWoe9YZWxBLrQA3qzHJivBGtfciMekk8m5").unwrap(),
        // ===== ORIGINAL CURATED US-EAST VALIDATORS =====
        // Jito validators (multiple stake pools, US-based infrastructure)
        Pubkey::from_str("J1to1yufRnoWn81KYg1XkTWzmKjnYSnmE2VY8DGUJ9Qv").unwrap(),
        Pubkey::from_str("J1to2NAwajc8hD6E6kujdQiPn1Bbt2mGKKZLY9kSQKdB").unwrap(),
        Pubkey::from_str("J1to3ouYE6kFgJwU9Cqc3RGXVH93nmzMCSnLNZLsUDh9").unwrap(),
        // Everstake US
        Pubkey::from_str("12ashmTiFStQ8RGUpi1BTCinJakVyDKWjRL6SWhnbxbT").unwrap(),
        // Certus One
        Pubkey::from_str("CertusDeBmqN8ZawdkxK5kFGMwBXdudvWHYwtNgNhvLu").unwrap(),
        // Dumi
        Pubkey::from_str("DumiCKHVqoCQKD8roLApzR5Fit8qGV5fVTEQmLYGN2ze").unwrap(),
        // Marinade (distributed US infrastructure)
        Pubkey::from_str("mrgn2vsQZuPNjS8DeUPNMHMo9VhqMDYWoqvDZVJGGRT").unwrap(),
        // Coinbase Cloud (US East)
        Pubkey::from_str("HBH4g3g9GQ8Gq7JNCPmvtEVPjLr8UQknTHXdNJ9jY8SV").unwrap(),
        // Figment (US infrastructure)
        Pubkey::from_str("CakcnaRDHka2gXyfbEd2d3xsvkJkqsLw2akB3zsN1D2S").unwrap(),
        // Blockdaemon US
        Pubkey::from_str("AkK5UBFwkx3F2P33Lzmtm3RdWNjnoAyipwPTNhfAUSkX").unwrap(),
        // ===== VALIDATORS.APP API - VERIFIED US-EAST DATACENTERS (2025-12-09) =====
        // Binance Staking (Ashburn, 10.7M SOL)
        Pubkey::from_str("3N7s9zXMZ4QqvHQR15t5GNHyqc89KduzMP7423eWiD5g").unwrap(),
        // High-stake validator (Newark, 3.8M SOL)
        Pubkey::from_str("8Pep3GmYiijRALqrMKpez92cxvF4YPTzoZg83uXh14pW").unwrap(),
        // Overclock (New York, 1.6M SOL)
        Pubkey::from_str("AS3nKBQfKs8fJ8ncyHrdvo4FDT6S8HMRhD75JjCcyr1t").unwrap(),
        // DoubleZero x Asymmetric Research (Newark, 1.3M SOL)
        Pubkey::from_str("DZVyygR326SerwzVivgqb8REnegEyU4teMkGQaBV6Bpw").unwrap(),
        // Cavey Cool (Newark, 908K SOL)
        Pubkey::from_str("CooLbbZy5Xmdt7DiHPQ3ss2uRXawnTXXVgpMS8E8jDzr").unwrap(),
        // stakefish 🐟 (Miami, 685K SOL)
        Pubkey::from_str("7VGU4ZwR1e1AFekqbqv2gvjeg47e1PwMPm4BfLt6rxNk").unwrap(),
        // Valigator (Newark, 448K SOL)
        Pubkey::from_str("gaToR246dheK1DGAMEqxMdBJZwU4qFyt7DzhSwAHFWF").unwrap(),
        // Solayer Validator (Ashburn, 437K SOL)
        Pubkey::from_str("SLaYv7tCwetrFGbPCRnqpHswG5qqKino78EYpbGF7xY").unwrap(),
        // High-stake (Newark, 418K SOL)
        Pubkey::from_str("HM5H6FAYWEMcm9PCXFbbiUFfFVLTN9UGy9AqmMQjdMRA").unwrap(),
        // Temporal Topaz (Newark, 383K SOL)
        Pubkey::from_str("7emL18Bnve7wbYE9Az7vYJjikxN6YPU81igf6rVU5FN8").unwrap(),
        // Maera (New York, 345K SOL)
        Pubkey::from_str("47hzyKgoz9Fu1HP4mR48tvAFj7akCFZKGzZqWo32sNqG").unwrap(),
        // Phantom Validator (Newark, 307K SOL)
        Pubkey::from_str("J2nUHEAgZFRyuJbFjdqPrAa9gyWDuc7hErtDQHPhsYRp").unwrap(),
        // Neumann (Newark, 301K SOL)
        Pubkey::from_str("FrtCZRajYfrdoZarPxCsUB6f66ga5DPSXGz4F7VKyfKP").unwrap(),
        // High-stake (New York, 253K SOL)
        Pubkey::from_str("A5ed2x2wEuHW8KsuhbSaTFU4CYT3WgHfQcwyQXz5SdDw").unwrap(),
        // High-stake (Newark, 250K SOL)
        Pubkey::from_str("G8hvpQDLe7hGgYtWYt4TJJEbGbgLFCBJYiMi9pMD9Kk2").unwrap(),
        // Mad Lads (Newark, 234K SOL)
        Pubkey::from_str("B6nDYYLc2iwYqY3zdmavMmU9GjUL2hf79MkufviM2bXv").unwrap(),
        // Bandito Stake (Miami, 226K SOL)
        Pubkey::from_str("CwSZ17woioM2bqEbaswZJYvx5pemN6t3shBcU6zqPHyG").unwrap(),
        // MX Validator (New York, 215K SOL)
        Pubkey::from_str("Mxv1Ubm71XoUvxrN3qjN8ii6Bh5b43NuuKywsWx6ox2").unwrap(),
        // Nodance 💃 00 (Atlanta, 172K SOL)
        Pubkey::from_str("B8Jud6Mqv1FSKpjBM2hPBqoPhGMAciX2UqR9WcepDF7F").unwrap(),
        // bloXroute US (New York, 156K SOL)
        Pubkey::from_str("DSzLJLUQD55sxaCsJBHLFSV1SYngMmT7oY8rLpFhyGgb").unwrap(),
        // Sunshine (Miami, 133K SOL)
        Pubkey::from_str("FGtsnE1HB4bBi6g4xAt5mvWtuC3qBPWPBgWVrnRmUiVH").unwrap(),
        // Vnode (New York, 130K SOL)
        Pubkey::from_str("3r5ZXC1yFqMmk8VwDdUJbEdPmZ8KZvEkzd5ThEYRetTk").unwrap(),
        // MonkeDAO (New York, 102K SOL)
        Pubkey::from_str("DfpdmTsSCBPxCDwZwgBMfjjV8mF8xHkGRcXP8dJBVmrq").unwrap(),
        // SOLAnode (Miami, 101K SOL)
        Pubkey::from_str("7rwEPPMQgHr3mn5nQX27SADt3tR8yucBgsNapYdrqRhB").unwrap(),
        // LightNode (Atlanta, 99K SOL)
        Pubkey::from_str("ViKLknQuks11DLEjZ7Y2aNYAAT7Q3NTKLGxs8rdnLVi").unwrap(),
        // Hayek (Miami, 99K SOL)
        Pubkey::from_str("HAYEKSWg2EY21k38St9X5yM7QMW6SunKDefs5SqYSFty").unwrap(),
        // OranG3cluB (Miami, 96K SOL)
        Pubkey::from_str("AY271jdvcyo5VzBiWsMGLEjpZFFrarq8FDydJHLmYgCG").unwrap(),
        // CyberAlex MEV (New York, 94K SOL)
        Pubkey::from_str("VaCdXKupamusfRsDf9Ai7e8Up36Z4f3MP6SqhnM7c76").unwrap(),
        // Saga DAO (Ashburn, 92K SOL)
        Pubkey::from_str("sagasJDjjAHND4hien3bbo5xXkzCT5Ss6nKjyUJ45aw").unwrap(),
        // SolX +Mev (Newark, 88K SOL)
        Pubkey::from_str("SzNm2zDpK3eJmSYR21pRaQ5b4wQ37ecds6det5YCZFP").unwrap(),
        // Waterloo Blockchain (New York, 66K SOL)
        Pubkey::from_str("gGQb6ZcDG7fsVLWTvXZ1LLoY348tc1zcP2wkh8kshrk").unwrap(),
        // Blockchain at Emory (New York, 65K SOL)
        Pubkey::from_str("ySxF6XaSFSwU46iJbgyh2rAW5jagLbYULPtWvZCshrk").unwrap(),
        // Hedgehog Spiky Validator (Ashburn, 61K SOL)
        Pubkey::from_str("3dXXxEaV4fZqw1PL7VezfDkiJV5W4WTtRjh2EHgKSthF").unwrap(),
        // Goldsand (Ashburn, 61K SOL)
        Pubkey::from_str("SANDhe6azby4EMLJd8N77QTk5K92n2tCi4NXba7iwuc").unwrap(),
        // Rutgers Blockchain (New York, 48K SOL)
        Pubkey::from_str("FH5SX1WUubW9nD1rweZMofCHGhEp8qUMx3PksLfshrk").unwrap(),
        // Villanova Crypto Club (New York, 48K SOL)
        Pubkey::from_str("c3wFuF1LCtsY5DGo7eBQCwLHhtZgAxNwNDrUCqashrk").unwrap(),
        // University of Minnesota Blockchain Club (New York, 47K SOL)
        Pubkey::from_str("Cw2b2ng2fa78ndCXHcJMT1pqvdGxUHu5EBEB8KBshrk").unwrap(),
        // Blockchain at Columbia (New York, 47K SOL)
        Pubkey::from_str("FRqYRkp3tUoBc62pnabeX8SVr6p3v6HM2tzT2NVshrk").unwrap(),
        // Princeton Blockchain (New York, 47K SOL)
        Pubkey::from_str("ksQuu3JAStVXUeDCHeW6QtVRHNMcdtgPyepUd3Yshrk").unwrap(),
        // Dalhousie Blockchain Society (New York, 47K SOL)
        Pubkey::from_str("T6bpj6H6fr9GyKe73DcNjPbGeSk8twDsiNcU4srshrk").unwrap(),
        // Blockchain at Georgia Tech (New York, 47K SOL)
        Pubkey::from_str("GLB3jUr5zuu79zFuEH3KP7boxtWsjHVyYRBK2SPshrk").unwrap(),
        // YYDS Validator (Newark, 46K SOL)
        Pubkey::from_str("DzPT1ZWDeURdTj38QBSceWnrpYFxZRBLPRXmUgHVDAGR").unwrap(),
        // Block Rush Validator (Newark, 18K SOL)
        Pubkey::from_str("FEjcS4JCTqitjzW4Zj3Va2ioZZKf7MqsCMnrrgGLSvjm").unwrap(),
        // Frankendancer-DoubleZero2 (Newark, 10K SOL)
        Pubkey::from_str("4T799AaK9YT7zBtVqYZEnCY5ihUF5XaEYemwr5EnozoQ").unwrap(),
        // test_validator_1 (New York, 7K SOL)
        Pubkey::from_str("7MKUXUiiVeWFwLaNyw5exkujXnen5yB77hQa54KLD98h").unwrap(),
        // InfStones (Ashburn, 6K SOL)
        Pubkey::from_str("37BPVW1Ne1XHrzK15xguAS2BTdobVfThDzTE2mv8SsnJ").unwrap(),
    ]
});

/// Leader schedule manager for 0slot trading
pub struct LeaderScheduleManager {
    rpc_client: RpcClient,
}

impl LeaderScheduleManager {
    /// Create a new leader schedule manager
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_client: RpcClient::new_with_timeout(rpc_url, Duration::from_secs(10)),
        }
    }

    /// Update the current slot (call this from ShredStream processor)
    pub fn update_current_slot(slot: u64) {
        let prev = CURRENT_SLOT.swap(slot, Ordering::SeqCst);
        if slot > prev + 10 {
            debug!("⚡ 0SLOT: Current slot updated {} -> {}", prev, slot);
        }
    }

    /// Get the current slot
    pub fn get_current_slot() -> u64 {
        CURRENT_SLOT.load(Ordering::SeqCst)
    }

    /// Check if the current leader is "local" (low latency for 0slot trading)
    ///
    /// Returns (is_local, leader_pubkey, latency_ms)
    pub async fn is_current_leader_local(&self) -> (bool, Option<Pubkey>, u64) {
        let current_slot = Self::get_current_slot();

        // Refresh leader schedule if needed
        if self.should_refresh_schedule(current_slot) {
            if let Err(e) = self.refresh_leader_schedule().await {
                warn!("⚡ 0SLOT: Failed to refresh leader schedule: {}", e);
            }
        }

        // Get leader for current slot
        let leader = {
            let cache = LEADER_SCHEDULE_CACHE.read().await;
            cache.get(&current_slot).cloned()
        };

        match leader {
            Some(leader_pubkey) => {
                let latency = self.get_leader_latency(&leader_pubkey);
                let is_local = latency < LOCAL_LEADER_LATENCY_THRESHOLD_MS;

                if is_local {
                    debug!(
                        "⚡ 0SLOT: LOCAL leader {} ({}ms) - TRADE!",
                        &leader_pubkey.to_string()[..8],
                        latency
                    );
                } else {
                    debug!(
                        "⚡ 0SLOT: REMOTE leader {} ({}ms) - SKIP",
                        &leader_pubkey.to_string()[..8],
                        latency
                    );
                }

                (is_local, Some(leader_pubkey), latency)
            }
            None => {
                debug!(
                    "⚡ 0SLOT: No leader info for slot {} - assuming remote",
                    current_slot
                );
                (false, None, 999)
            }
        }
    }

    /// Check if we should refresh the leader schedule
    fn should_refresh_schedule(&self, current_slot: u64) -> bool {
        let last_fetch = LAST_SCHEDULE_FETCH_SLOT.load(Ordering::SeqCst);
        current_slot > last_fetch + LEADER_SCHEDULE_REFRESH_SLOTS
    }

    /// Refresh the leader schedule from RPC
    async fn refresh_leader_schedule(&self) -> Result<()> {
        let start = Instant::now();

        // Get leader schedule for current epoch
        let slot_leaders = self
            .rpc_client
            .get_slot_leaders(
                Self::get_current_slot(),
                100, // Get next 100 slots (~40 seconds)
            )
            .map_err(|e| anyhow!("Failed to get leader schedule: {}", e))?;

        // Update cache
        let current_slot = Self::get_current_slot();
        {
            let mut cache = LEADER_SCHEDULE_CACHE.write().await;
            cache.clear();

            for (offset, leader) in slot_leaders.iter().enumerate() {
                cache.insert(current_slot + offset as u64, *leader);
            }
        }

        LAST_SCHEDULE_FETCH_SLOT.store(current_slot, Ordering::SeqCst);

        info!(
            "⚡ 0SLOT: Leader schedule refreshed ({} slots) in {}ms",
            slot_leaders.len(),
            start.elapsed().as_millis()
        );

        Ok(())
    }

    /// Get estimated latency for a leader validator
    /// Uses cached measurements or falls back to known US-East validators list
    fn get_leader_latency(&self, leader: &Pubkey) -> u64 {
        // Check cache first
        if let Some(entry) = VALIDATOR_LATENCY_CACHE.get(leader) {
            let (latency, measured_at) = *entry;
            if measured_at.elapsed().as_secs() < LATENCY_CACHE_DURATION_SECS {
                return latency;
            }
        }

        // Fall back to known US-East validators
        if US_EAST_VALIDATORS.contains(leader) {
            // Known US-East validator - assume low latency
            VALIDATOR_LATENCY_CACHE.insert(*leader, (20, Instant::now()));
            return 20;
        }

        // Unknown validator - assume high latency (conservative)
        // In production, would measure actual latency via ping
        VALIDATOR_LATENCY_CACHE.insert(*leader, (200, Instant::now()));
        200
    }

    /// Measure actual latency to a validator's gossip endpoint
    /// TODO: Implement actual ping measurement
    #[allow(dead_code)]
    async fn measure_validator_latency(&self, _leader: &Pubkey) -> u64 {
        // Placeholder - in production would:
        // 1. Get validator's gossip IP from cluster info
        // 2. Send UDP ping packets
        // 3. Measure round-trip time
        // 4. Cache result

        // For now, return default
        100
    }

    /// Start background task to refresh leader schedule
    pub fn start_schedule_refresh_task(rpc_url: String) {
        tokio::spawn(async move {
            let manager = LeaderScheduleManager::new(rpc_url);

            info!("⚡ 0SLOT: Starting leader schedule refresh task");

            loop {
                // Refresh every 30 seconds
                tokio::time::sleep(Duration::from_secs(30)).await;

                if let Err(e) = manager.refresh_leader_schedule().await {
                    warn!("⚡ 0SLOT: Leader schedule refresh error: {}", e);
                }
            }
        });
    }

    /// Get stats about the leader schedule cache
    pub async fn get_cache_stats() -> (usize, u64, usize) {
        let schedule_size = LEADER_SCHEDULE_CACHE.read().await.len();
        let last_fetch = LAST_SCHEDULE_FETCH_SLOT.load(Ordering::SeqCst);
        let latency_cache_size = VALIDATOR_LATENCY_CACHE.len();

        (schedule_size, last_fetch, latency_cache_size)
    }
}

/// Helper function to check if we should trade this opportunity
/// Call this BEFORE executing sandwich logic
pub async fn should_trade_current_slot(rpc_url: &str) -> bool {
    let manager = LeaderScheduleManager::new(rpc_url.to_string());

    // CRITICAL FIX: First fetch current slot from RPC and update the global tracker
    // Without this, CURRENT_SLOT stays at 0 and leader schedule lookup fails!
    match manager.rpc_client.get_slot() {
        Ok(slot) => {
            LeaderScheduleManager::update_current_slot(slot);
            debug!("⚡ 0SLOT: Updated current slot to {}", slot);
        }
        Err(e) => {
            warn!(
                "⚡ 0SLOT: Failed to get current slot from RPC: {} - skipping",
                e
            );
            return false;
        }
    }

    let (is_local, leader, latency) = manager.is_current_leader_local().await;

    if is_local {
        info!(
            "⚡ 0SLOT: TRADING - local leader {:?} ({}ms)",
            leader.map(|l| l.to_string()[..8].to_string()),
            latency
        );
        true
    } else {
        debug!(
            "⚡ 0SLOT: SKIPPING - remote leader {:?} ({}ms)",
            leader.map(|l| l.to_string()[..8].to_string()),
            latency
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_update() {
        LeaderScheduleManager::update_current_slot(12345);
        assert_eq!(LeaderScheduleManager::get_current_slot(), 12345);

        LeaderScheduleManager::update_current_slot(12346);
        assert_eq!(LeaderScheduleManager::get_current_slot(), 12346);
    }

    #[test]
    fn test_us_east_validators_loaded() {
        assert!(!US_EAST_VALIDATORS.is_empty());
        // Should have at least the Jito validators
        assert!(US_EAST_VALIDATORS.len() >= 3);
    }
}
