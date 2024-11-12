# DNSpeedTest

A fast, lightweight DNS resolver benchmarking tool written in Rust that measures the performance of popular DNS providers.

## Features

- Tests multiple popular DNS providers including Google, Cloudflare, Quad9, OpenDNS, and more
- Measures average response time, minimum and maximum latency
- Calculates success rate for DNS queries
- Tests against commonly accessed domains
- Provides detailed performance metrics in an easy-to-read format
- Initial connection latency testing for each provider

## Build from source

1. Make sure you have Rust installed on your system
2. Clone this repository
```
git clone https://github.com/0xgingi/dnsspeedtest
cd dnspeedtest
```
3. Build and run:
```
cargo build --release
cargo run
```
## Configuration

The tool comes pre-configured with several popular DNS providers and test domains. You can modify these in the source code:

- `DNS_PROVIDERS`: List of DNS providers to test
- `TEST_DOMAINS`: List of domains to query during testing
- `TEST_ROUNDS`: Number of test iterations (default: 3)