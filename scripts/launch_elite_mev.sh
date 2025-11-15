#!/bin/bash

# ELITE MEV Bot Launch Script
# Optimized for maximum performance and monitoring

set -e

echo "🚀 ELITE MEV Bot Launch System"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Check if .env file exists
if [ ! -f ".env" ]; then
    echo -e "${RED}❌ .env file not found!${NC}"
    echo -e "${YELLOW}💡 Copy .env.elite_example to .env and configure:${NC}"
    echo "   cp .env.elite_example .env"
    echo "   nano .env"
    exit 1
fi

# Load environment variables
source .env

# Validate required variables
if [ -z "$SHREDS_ENDPOINT" ] || [ -z "$JUPITER_API_KEY" ]; then
    echo -e "${RED}❌ Missing required environment variables!${NC}"
    echo -e "${YELLOW}Required: SHREDS_ENDPOINT, JUPITER_API_KEY${NC}"
    exit 1
fi

# Display configuration
echo -e "${GREEN}✅ Configuration Loaded${NC}"
echo -e "${CYAN}📡 ShredStream: $SHREDS_ENDPOINT${NC}"
echo -e "${CYAN}🔑 Jupiter API: ${JUPITER_API_KEY:0:8}***${NC}"
echo -e "${CYAN}💰 Capital: ${CAPITAL_SOL:-4.0} SOL${NC}"
echo -e "${CYAN}🎯 Risk Level: ${RISK_LEVEL:-MODERATE}${NC}"
echo -e "${CYAN}💎 Min Profit: ${MIN_PROFIT_SOL:-0.15} SOL${NC}"

# Build the bot if needed
echo -e "${YELLOW}🔨 Building ELITE MEV Bot...${NC}"
if ! cargo build --bin elite_mev_bot --release 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Release build failed, trying debug build...${NC}"
    if ! cargo build --bin elite_mev_bot; then
        echo -e "${RED}❌ Build failed! Check your Rust installation and dependencies.${NC}"
        exit 1
    fi
    BINARY_PATH="./target/debug/elite_mev_bot"
else
    BINARY_PATH="./target/release/elite_mev_bot"
fi

echo -e "${GREEN}✅ Build successful: $BINARY_PATH${NC}"

# Create logs directory
mkdir -p logs

# Generate log filename with timestamp
LOG_FILE="logs/elite_mev_$(date +%Y%m%d_%H%M%S).log"

echo -e "${PURPLE}📊 Performance Monitoring:${NC}"
echo -e "${CYAN}  • Real-time metrics every 10 seconds${NC}"
echo -e "${CYAN}  • Adaptive parameter optimization${NC}"
echo -e "${CYAN}  • Comprehensive performance analytics${NC}"
echo -e "${CYAN}  • Log file: $LOG_FILE${NC}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}🎯 LAUNCHING ELITE MEV BOT${NC}"
echo -e "${YELLOW}💡 Press Ctrl+C to stop gracefully${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Launch with logging
$BINARY_PATH 2>&1 | tee "$LOG_FILE"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}👋 ELITE MEV Bot session ended${NC}"
echo -e "${CYAN}📊 Log saved to: $LOG_FILE${NC}"
echo -e "${YELLOW}💡 Check logs for performance analysis and optimization tips${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"