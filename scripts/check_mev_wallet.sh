#!/bin/bash
# Check MEV Bot Wallet Balance and Info

set -e

# Load environment
if [ -f .env.mev_production ]; then
    export $(cat .env.mev_production | grep -v '^#' | xargs)
else
    echo "❌ Error: .env.mev_production not found"
    exit 1
fi

echo "🔐 MEV Bot Wallet Information"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Create a temporary Rust program to derive the public key
cat > /tmp/check_wallet.rs << 'EOF'
use solana_sdk::signature::{Keypair, Signer};
use bs58;

fn main() {
    let private_key_str = std::env::var("WALLET_PRIVATE_KEY")
        .expect("WALLET_PRIVATE_KEY not set");

    // Decode base58 private key
    let private_key_bytes = bs58::decode(&private_key_str)
        .into_vec()
        .expect("Invalid base58");

    // Create keypair
    let keypair = Keypair::from_bytes(&private_key_bytes)
        .expect("Invalid keypair bytes");

    println!("Public Key: {}", keypair.pubkey());
}
EOF

# Compile and run
cd /tmp
rustc check_wallet.rs --edition 2021 -L ~/.cargo/registry/src/*/solana-sdk-*/target/release/deps 2>/dev/null || {
    # If rustc fails, use a simpler Python approach
    python3 << 'PYEOF'
import os
import base58
from nacl.signing import SigningKey

private_key_b58 = os.environ['WALLET_PRIVATE_KEY']
private_key_bytes = base58.b58decode(private_key_b58)

# Solana keypair is 64 bytes (32 private + 32 public)
signing_key = SigningKey(private_key_bytes[:32])
public_key = signing_key.verify_key
public_key_b58 = base58.b58encode(bytes(public_key)).decode('ascii')

print(f"Public Key: {public_key_b58}")
PYEOF
} || {
    echo "⚠️  Could not derive public key (missing dependencies)"
    echo "Install: pip3 install base58 PyNaCl"
    PUBLIC_KEY="<derive manually>"
}

# Get wallet address (if we got it from above)
PUBLIC_KEY=$(./check_wallet 2>/dev/null | grep "Public Key:" | cut -d' ' -f3 || echo "unknown")

if [ "$PUBLIC_KEY" != "unknown" ]; then
    echo "📫 Wallet Address: $PUBLIC_KEY"
    echo ""

    # Check balance
    echo "💰 Checking SOL balance..."
    BALANCE=$(solana balance $PUBLIC_KEY 2>/dev/null || echo "Error: solana CLI not available")
    echo "   Balance: $BALANCE"
    echo ""
fi

# Show configuration
echo "⚙️  Configuration:"
echo "   Paper Trading: $PAPER_TRADING"
echo "   Real Trading: $ENABLE_REAL_TRADING"
echo "   Max Position: $MAX_POSITION_SIZE_SOL SOL"
echo "   Min Profit: $MIN_PROFIT_THRESHOLD_SOL SOL"
echo ""

# Safety check
if [ "$ENABLE_REAL_TRADING" = "true" ]; then
    echo "⚠️  WARNING: REAL TRADING IS ENABLED!"
    echo "   Bot will execute real trades with real money!"
else
    echo "✅ SAFE: Paper trading mode enabled"
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
