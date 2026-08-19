# hl-md-handler
A L2 orderbook data handler for Hyperliquid DEX using Quicknode's gRPC API

### Features & Design Choices
- Uses `Tokio` and `Tonic` for async runtime and gRPC implementation
- Uses `tokio::sync::mpsc` as queue between producer (Feed) and consumer (Engine)
- Uses `tracing-appender` for non-blocking logging
- Connects to Quicknode's Hyperliquid API, requiring gRPC endpoint and auth token (I can provide a temporary set for demo purpose)
- Book levels are processed in fixed-size, sorted arrays. Uses linear search for level searching favoring top levels, and `copy_within()` for updating levels in place
- [Faster parsing](#benchmarks) from decimal string to `u64`
- Supports multiple perpetual symbols

### Quickstart
Configuration: Put the gRPC endpoint, auth token, and perp symbols to subscribe in `config/config.toml`

At the project root:
```
$ cargo run --release
```

### Architecture
todo

### File Structure
todo

### Latency Measurement
todo

### Benchmarks
A simple benchmark using Criterion for `Engine::parse_to_u64_with_mul()`, with `s.parse::<f64>() as u64` as the baseline:
```
parse_str_to_u64/engine_parse/123.456       time:   [2.3882 ns 2.3949 ns 2.4032 ns]
parse_str_to_u64/f64_parse/123.456          time:   [7.3538 ns 7.3929 ns 7.4282 ns]

parse_str_to_u64/engine_parse/123           time:   [1.5925 ns 1.5953 ns 1.5990 ns]
parse_str_to_u64/f64_parse/123              time:   [4.7746 ns 4.7750 ns 4.7755 ns]

parse_str_to_u64/engine_parse/0.00123456    time:   [1.9513 ns 1.9735 ns 2.0037 ns]
parse_str_to_u64/f64_parse/0.00123456       time:   [5.8745 ns 5.8934 ns 5.9167 ns]

parse_str_to_u64/engine_parse/123456.7      time:   [2.9178 ns 2.9217 ns 2.9297 ns]
parse_str_to_u64/f64_parse/123456.7         time:   [7.3343 ns 7.4255 ns 7.5393 ns]
```

### Limitations
- Support perpetuals only
- No. of book levels (20) is not configurable

### Potential Improvements
- Book staleness check against RESTful
- Support multiple exchanges using the Feed trait
- Write to shared memory for IPC with actual clients

This repository is for learning purpose only
