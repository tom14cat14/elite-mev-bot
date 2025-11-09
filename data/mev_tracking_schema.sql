-- MEV Bot Tracking Database Schema
-- Tracks all detected opportunities, executions, and profitability across 6 DEX types

-- Main opportunities table
CREATE TABLE IF NOT EXISTS opportunities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    signature TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    dex_name TEXT NOT NULL,  -- Raydium_AMM_V4, Raydium_CLMM, Raydium_CPMM, Orca_Whirlpools, Meteora_DLMM, PumpSwap
    pool_address TEXT,
    victim_swap_size_sol REAL NOT NULL,
    estimated_profit_sol REAL,
    status TEXT NOT NULL,  -- detected, submitted, confirmed, skipped, failed
    skip_reason TEXT,  -- Why opportunity was skipped
    slot BIGINT,

    -- Execution details (NULL if not executed)
    executed_at DATETIME,
    actual_profit_sol REAL,
    fees_paid_sol REAL,
    jito_tip_sol REAL,
    position_size_sol REAL,
    bundle_id TEXT,

    -- On-chain confirmation tracking
    confirmed_on_chain BOOLEAN DEFAULT 0,  -- Whether bundle landed on chain
    confirmed_at DATETIME,  -- When confirmation detected
    confirmation_block BIGINT,  -- Block number where confirmed
    confirmation_signature TEXT,  -- Actual transaction signature on chain

    -- Performance metrics
    detection_latency_ms REAL,
    execution_latency_ms REAL,

    UNIQUE(signature)  -- Prevent duplicate opportunities
);

-- Index for fast queries
CREATE INDEX IF NOT EXISTS idx_timestamp ON opportunities(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_dex_name ON opportunities(dex_name);
CREATE INDEX IF NOT EXISTS idx_status ON opportunities(status);
CREATE INDEX IF NOT EXISTS idx_date ON opportunities(DATE(timestamp));

-- Daily summary table (for faster dashboard queries)
CREATE TABLE IF NOT EXISTS daily_stats (
    date DATE PRIMARY KEY,
    total_detected INTEGER DEFAULT 0,
    total_executed INTEGER DEFAULT 0,
    total_profit_sol REAL DEFAULT 0,
    total_fees_sol REAL DEFAULT 0,

    -- Per-DEX breakdown
    raydium_v4_detected INTEGER DEFAULT 0,
    raydium_v4_executed INTEGER DEFAULT 0,
    raydium_v4_profit_sol REAL DEFAULT 0,

    raydium_clmm_detected INTEGER DEFAULT 0,
    raydium_cpmm_detected INTEGER DEFAULT 0,
    orca_detected INTEGER DEFAULT 0,
    meteora_detected INTEGER DEFAULT 0,
    pumpswap_detected INTEGER DEFAULT 0,

    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Configuration snapshots (track bot config changes)
CREATE TABLE IF NOT EXISTS config_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    mode TEXT NOT NULL,  -- multi-dex or pumpfun
    paper_trading BOOLEAN NOT NULL,
    min_swap_size_sol REAL NOT NULL,
    max_swap_size_sol REAL NOT NULL,
    min_profit_sol REAL NOT NULL,
    position_size_strategy TEXT,
    jito_tip_strategy TEXT
);

-- Performance metrics (track bot health)
CREATE TABLE IF NOT EXISTS performance_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    avg_detection_latency_ms REAL,
    avg_execution_latency_ms REAL,
    jito_success_rate REAL,
    opportunities_per_minute REAL,
    wallet_balance_sol REAL
);
