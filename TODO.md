# TODO

## Open

### Codegen: extend TOML schema to generate C++/Go/FFI types
- [ ] `build.rs` emits C header (`thetadatadx_types.h`) from `endpoint_schema.toml`
- [ ] `build.rs` emits C++ structs + DataTable parsers from TOML
- [ ] `build.rs` emits Go structs + DataTable parsers from TOML
- [ ] `build.rs` emits Python dict converters from TOML
- [ ] `build.rs` emits FFI JSON serializers from TOML
- [ ] One TOML change propagates to all 6 languages automatically

### Integration testing (requires live server)
- [ ] Test terminal version negotiation -- does the server care about `terminal_git_commit`?
- [ ] FPSS TLS handshake to `nj-a.thetadata.us:20000`
- [ ] FPSS CREDENTIALS -> METADATA round-trip
- [ ] FPSS sustained volume during market hours (no dropped messages)
- [ ] FPSS reconnection: kill TCP mid-stream, verify re-subscribe
- [ ] FPSS rate limiting: trigger `TooManyRequests`, verify 130s backoff

### Build verification
- [ ] Go SDK: verify CGo builds on Linux + macOS
- [ ] C++ SDK: verify CMake build with FFI static library

### Architecture (nice-to-have)
- [ ] `tracing` spans on all network operations
- [ ] Metrics (request count, latency histograms, reconnect count)
- [ ] Runtime config from `config.toml` / `config.properties`

## Done

### v3.1.0 -- Audit fixes
- [x] Go SDK priceToFloat encoding (was fundamentally wrong)
- [x] Python docs event["type"] -> event["kind"]
- [x] Price::new panic -> debug_assert + clamp
- [x] C++ FpssClient missing unsubscribe_quotes
- [x] FFI FPSS mutex poison recovery
- [x] Credentials.password pub -> pub(crate)
- [x] WS handler OPEN_INTEREST + FULL_TRADES dispatch
- [x] C++ MarketValueTick/CalendarDay/InterestRateTick field parity

### v3.0.0 -- Unified ThetaDataDx client
- [x] ThetaDataDx::connect() single entry point
- [x] Lazy FPSS via start_streaming()
- [x] Explicit Drop impl
- [x] FFI tdx_unified_historical (repr(transparent))
- [x] Python _df DataFrame methods on ThetaDataDx
- [x] All docs/notebooks/READMEs updated
- [x] Server stop_streaming on shutdown

### Typed endpoints + TOML codegen
- [x] 9 new tick types (TradeQuoteTick, GreeksTick, IvTick, etc.)
- [x] All 31 raw_endpoint! converted to parsed_endpoint!
- [x] raw_endpoint! macro removed
- [x] endpoint_schema.toml as single source of truth
- [x] build.rs generates Rust structs + parsers from TOML

### SDK completeness
- [x] All 61 endpoints in Rust, Python, Go, C++, C FFI
- [x] All endpoints return native typed structures (zero raw DataTable/JSON)
- [x] Python: FPSS streaming, pandas DataFrames, _df variants
- [x] Go: 9 typed structs, FpssClient
- [x] C++: 9 typed structs, FpssClient with RAII
- [x] Published to crates.io and PyPI

### Performance
- [x] repr(C, align(64)) on tick types
- [x] Cached QueryInfo template
- [x] Precomputed column indices
- [x] Thread-local zstd decompressor
- [x] LMAX Disruptor ring buffer (disruptor-rs v4)
- [x] Criterion benchmarks
- [x] Streaming for_each_chunk callback
- [x] Horner-form norm_cdf

### Wire protocol parity
- [x] All 60 gRPC RPCs + 1 range variant
- [x] FPSS framing, FIT encoding, delta compression
- [x] Contract serialization (fixed in v1.2.0)
- [x] Greeks formulas (fixed in v1.2.0)
- [x] OHLCVC-from-trade derivation
- [x] Auth flow (Nexus + Wix)
- [x] Dynamic 2^tier concurrent requests
