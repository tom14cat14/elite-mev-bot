-- MEV Bot Opportunity Tracking Database Schema
-- Created: 2025-11-10
-- Purpose: Track detected opportunities, execution attempts, and results

CREATE TABLE IF NOT EXISTS opportunities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Detection metadata
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    signature TEXT NOT NULL,  -- Victim transaction signature
    dex_name TEXT NOT NULL,
    pool_address TEXT NOT NULL,

    -- Opportunity details
    estimated_sol_value REAL NOT NULL,
    estimated_price_impact REAL,
    quality_score REAL,

    -- Execution status
    status TEXT NOT NULL DEFAULT 'detected',  -- detected, skipped, attempted, executed, failed
    skip_reason TEXT,

    -- Position sizing
    position_size_sol REAL,
    estimated_gross_profit REAL,
    estimated_net_profit REAL,
    total_fees REAL,

    -- JITO submission
    jito_tip_lamports INTEGER,
    bundle_submitted_at TIMESTAMP,
    our_signature TEXT,  -- Our transaction signature (if executed)

    -- Execution results
    executed_at TIMESTAMP,
    actual_profit_sol REAL,
    execution_error TEXT,

    -- Token details
    token_mint TEXT,
    token_amount REAL,

    UNIQUE(signature, dex_name)
);

CREATE INDEX IF NOT EXISTS idx_detected_at ON opportunities(detected_at);
CREATE INDEX IF NOT EXISTS idx_status ON opportunities(status);
CREATE INDEX IF NOT EXISTS idx_signature ON opportunities(signature);
CREATE INDEX IF NOT EXISTS idx_dex_name ON opportunities(dex_name);

-- Summary statistics view
CREATE VIEW IF NOT EXISTS daily_stats AS
SELECT
    DATE(detected_at) as date,
    COUNT(*) as total_opportunities,
    SUM(CASE WHEN status = 'detected' THEN 1 ELSE 0 END) as detected,
    SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END) as skipped,
    SUM(CASE WHEN status = 'attempted' THEN 1 ELSE 0 END) as attempted,
    SUM(CASE WHEN status = 'executed' THEN 1 ELSE 0 END) as executed,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
    SUM(actual_profit_sol) as total_profit_sol,
    AVG(estimated_price_impact) as avg_price_impact
FROM opportunities
GROUP BY DATE(detected_at)
ORDER BY date DESC;
