# Project-Specific Instructions for MEV_Bot

## 🎯 Project Type: Rust Trading Bot

### Architecture Overview
- **Language**: Rust 1.70+
- **Type**: [MEV Bot / Arbitrage Bot / Other]
- **Framework**: Tokio (async runtime)
- **Data Source**: [ShredStream / ERPC / Helius]
- **Trading Platform**: Solana blockchain

### Code Standards

#### Rust Style
- **Formatter**: rustfmt (default settings)
- **Linter**: clippy (all warnings as errors)
- **Edition**: 2021
- **MSRV**: 1.70.0

#### Testing
- **Framework**: Built-in Rust tests + criterion for benchmarks
- **Coverage**: cargo-tarpaulin
- **Location**: `tests/` directory + inline `#[test]` modules
- **Command**: `cargo test --all-features`

#### Security
- **NO unsafe blocks** unless absolutely necessary and documented
- **NO unwrap() in production code** - use proper error handling
- **NO .expect() without detailed messages**
- **All errors must implement std::error::Error**

### Project Structure
```
MEV_Bot/
├── src/
│   ├── main.rs            # Entry point
│   ├── lib.rs             # Library root
│   ├── config/            # Configuration
│   ├── data/              # Data fetching
│   ├── strategy/          # Trading strategies
│   ├── execution/         # Trade execution
│   └── utils/             # Utilities
├── tests/                 # Integration tests
├── benches/               # Benchmarks
├── Cargo.toml            # Dependencies
└── Cargo.lock            # Locked versions
```

### Critical Rules

#### 1. Error Handling
```rust
// ❌ Bad
let value = risky_operation().unwrap();

// ✅ Good
let value = risky_operation()
    .context("Failed to perform risky operation")?;
```

#### 2. Async Best Practices
```rust
// ❌ Bad - blocking in async
async fn bad_example() {
    std::thread::sleep(Duration::from_secs(1));
}

// ✅ Good - proper async
async fn good_example() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

#### 3. Resource Management
```rust
// ✅ Always use RAII
// ✅ Prefer Arc<Mutex<T>> over unsafe sharing
// ✅ Use channels for cross-thread communication
```

### Development Workflow

1. **Before Making Changes**
   ```bash
   # Pull latest
   git pull origin main

   # Create feature branch
   git checkout -b feature/your-feature

   # Build to check dependencies
   cargo build
   ```

2. **During Development**
   ```bash
   # Format code
   cargo fmt

   # Run clippy
   cargo clippy -- -D warnings

   # Run tests
   cargo test

   # Check for outdated deps
   cargo outdated
   ```

3. **Before Committing**
   ```bash
   # Full check
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test --all-features
   cargo build --release

   # Security audit
   cargo audit
   ```

### Common Codacy Issues & Fixes

#### Issue: "This expression borrows a reference that is immediately dereferenced"
```rust
// ❌ Bad
fn process(&value: &String) {
    println!("{}", value);
}

// ✅ Good
fn process(value: &String) {
    println!("{}", value);
}
```

#### Issue: "You should consider adding a 'Default' implementation"
```rust
// ❌ Bad
impl MyStruct {
    pub fn new() -> Self {
        Self { field: 0 }
    }
}

// ✅ Good
#[derive(Default)]
struct MyStruct {
    field: i32,
}

// Or implement manually
impl Default for MyStruct {
    fn default() -> Self {
        Self::new()
    }
}
```

#### Issue: "Unnecessary lifetime parameter"
```rust
// ❌ Bad
fn process<'a>(data: &'a str) -> &'a str {
    data
}

// ✅ Good - lifetime elision
fn process(data: &str) -> &str {
    data
}
```

#### Issue: "Use of 'unwrap' in production code"
```rust
// ❌ Bad
let config = load_config().unwrap();

// ✅ Good
let config = load_config()
    .context("Failed to load configuration file")?;
```

### Performance Best Practices

#### 1. Avoid Allocations
```rust
// ❌ Bad - allocates String every time
fn format_message(id: u64) -> String {
    format!("Message: {}", id)
}

// ✅ Good - use &str when possible
fn format_message(id: u64, buf: &mut String) {
    use std::fmt::Write;
    write!(buf, "Message: {}", id).unwrap();
}
```

#### 2. Use Appropriate Collections
```rust
// ✅ Vec for sequential access
// ✅ HashMap for O(1) lookups
// ✅ BTreeMap for ordered iteration
// ✅ HashSet for unique elements
```

#### 3. Minimize Cloning
```rust
// ❌ Bad - unnecessary clone
fn process(data: Vec<String>) {
    let copy = data.clone();
    println!("{:?}", copy);
}

// ✅ Good - borrow instead
fn process(data: &[String]) {
    println!("{:?}", data);
}
```

### Trading Bot Specifics

#### 1. JITO Integration
- ✅ Use base58 encoding (NOT base64)
- ✅ Implement dynamic tipping (save 99.3% on costs)
- ✅ Rate limit: 1 bundle per 1.1 seconds
- ✅ Handle 429 errors with exponential backoff

#### 2. ShredStream Integration
- ✅ Use gRPC-over-HTTPS (not pure gRPC)
- ✅ Endpoint: `grpc.erpc.cloud:443`
- ✅ Include metadata: `("x-erpc-key", API_KEY)`
- ✅ Handle reconnections automatically

#### 3. Transaction Building
```rust
// ✅ Always include proper error handling
// ✅ Use VersionedTransaction for recent features
// ✅ Simulate before sending
// ✅ Wait for confirmation (max 30s)
```

### Emergency Procedures

#### Kill Switch
```bash
# Stop bot immediately
tmux kill-session -t [SESSION_NAME]

# Or find and kill process
ps aux | grep [BOT_NAME]
kill -9 <PID>
```

#### Rollback
```bash
# Revert to last working version
git log --oneline
git reset --hard <commit-hash>
cargo build --release
```

### Compilation Checklist

Before deploying:
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `cargo build --release` succeeds
- [ ] `cargo audit` shows no vulnerabilities
- [ ] Paper trading validates strategy
- [ ] Wallet funded with minimum balance

### Contact & Resources
- **Main Doc**: `/home/tom14cat14/CLAUDE.md`
- **Architecture**: See bot-specific CLAUDE.md
- **JITO Setup**: `/home/tom14cat14/JITO_SETUP.md`
- **ShredStream Setup**: `/home/tom14cat14/SHREDSTREAM_SETUP.md`

---
**Last Updated**: 2025-11-15
**Codacy Integration**: Active
**Auto-Fix**: Enabled (rustfmt, clippy)
