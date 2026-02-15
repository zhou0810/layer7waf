# Layer 7 WAF

A high-performance Layer 7 Web Application Firewall built with [Pingora](https://github.com/cloudflare/pingora) (Rust proxy framework) and [Coraza](https://github.com/corazawaf/coraza) (OWASP CRS-compatible WAF engine).

## Architecture

```
                    ┌─────────────────────────────────────────────┐
                    │              Layer 7 WAF                     │
  Client ──HTTP──►  │                                              │
                    │  ┌──────────┐  ┌────────┐  ┌────────────┐  │
                    │  │ Pingora  │  │ Coraza │  │ Admin API  │  │
                    │  │ Proxy    ├─►│ Bridge ├─►│ (Axum)     │  │
                    │  │ Engine   │  │ (FFI)  │  │ + REST API │  │
                    │  └────┬─────┘  └────────┘  └────────────┘  │
                    │       │                                      │
                    │  ┌────┴─────┐  ┌──────────┐  ┌──────────┐  │
                    │  │ Rate     │  │ IP       │  │ Metrics  │  │
                    │  │ Limiter  │  │ Reputa-  │  │ (Prom)   │  │
                    │  │          │  │ tion     │  │          │  │
                    │  └──────────┘  └──────────┘  └──────────┘  │
                    └──────────────────┬──────────────────────────┘
                                       │
                                       ▼
                                  Upstream Servers
```

### Request Lifecycle

```
1. request_filter()   → IP check → Rate limit → Coraza request headers/body
2. upstream_peer()    → Select upstream (weighted round-robin)
3. response_filter()  → Coraza response headers/body check
4. logging()          → Structured JSON log, Prometheus metrics
```

## Features

- **WAF Engine**: Coraza WAF via Go FFI bridge with OWASP CRS compatibility
- **Rate Limiting**: Token bucket and sliding window algorithms with per-IP tracking
- **IP Reputation**: CIDR prefix trie for fast blocklist/allowlist lookups with hot-reload
- **Reverse Proxy**: Weighted round-robin upstream selection via Pingora
- **Admin REST API**: Health, metrics, config, rules management, audit logs
- **Observability**: Prometheus metrics, structured JSON logging

## Project Structure

```
layer7waf/
├── crates/
│   ├── proxy/          # Main binary - Pingora ProxyHttp pipeline
│   ├── coraza/         # Coraza WAF FFI bridge (Go → C shared lib → Rust)
│   ├── rate-limit/     # Token bucket & sliding window rate limiters
│   ├── ip-reputation/  # CIDR prefix trie for IP blocklist/allowlist
│   ├── admin/          # Axum REST API server
│   └── common/         # Shared config structs and error types
├── config/             # YAML configuration files
├── docker/             # Dockerfile and docker-compose
├── rules/              # WAF rule files (OWASP CRS)
└── tests/              # Integration and E2E tests
```

## Prerequisites

- **Rust** >= 1.77
- **Go** >= 1.22
- **CMake** (required by Pingora's dependencies)

## Build

```bash
cargo build --release
```

This automatically compiles the Go Coraza bridge into `libcoraza_bridge.so` via `build.rs`.

## Run

```bash
# Set library path for the Go shared library
export LD_LIBRARY_PATH=crates/coraza/go:$LD_LIBRARY_PATH

# Run with default config
./target/release/layer7waf config/layer7waf.yaml
```

The proxy listens on `0.0.0.0:8080` and the admin API on `127.0.0.1:9090` by default.

## Configuration

Edit `config/layer7waf.yaml`:

```yaml
server:
  listen: ["0.0.0.0:8080"]
  admin:
    listen: "127.0.0.1:9090"

upstreams:
  - name: backend
    servers:
      - addr: "127.0.0.1:8000"
        weight: 1

routes:
  - path_prefix: "/"
    upstream: backend
    waf:
      enabled: true
      mode: block          # block | detect | off

waf:
  rules:
    - "/path/to/owasp-crs/**/*.conf"
  request_body_limit: 13107200

rate_limit:
  enabled: true
  default_rps: 100
  default_burst: 200

ip_reputation:
  blocklist: "/path/to/blocklist.txt"
  allowlist: "/path/to/allowlist.txt"
```

## Admin API

| Endpoint | Method | Description |
|---|---|---|
| `/api/health` | GET | Health status and uptime |
| `/api/metrics` | GET | Prometheus metrics |
| `/api/config` | GET | Current running config |
| `/api/config` | PUT | Update config |
| `/api/rules` | GET | List WAF rules |
| `/api/rules` | POST | Add custom rule |
| `/api/rules/:id` | DELETE | Remove custom rule |
| `/api/rules/test` | POST | Test rule against sample request |
| `/api/logs` | GET | Query audit logs |
| `/api/stats` | GET | Traffic statistics |

```bash
# Check health
curl http://localhost:9090/api/health

# View stats
curl http://localhost:9090/api/stats

# Add a custom WAF rule
curl -X POST http://localhost:9090/api/rules \
  -H 'Content-Type: application/json' \
  -d '{"rule":"SecRule ARGS \"@contains test\" \"id:1001,phase:1,deny,status:403\""}'
```

## Docker

```bash
cd docker
docker compose up
```

This starts the WAF proxy on port 8080 and the admin API on port 9090, with an nginx upstream backend.

## Testing

```bash
# Run unit tests (30 tests)
cargo test --workspace

# Run E2E tests (requires running docker-compose stack)
./tests/e2e/test_waf.sh
```

## Roadmap

- **Phase 1** (current): WAF core, rate limiting, IP reputation, admin API
- **Phase 2**: Web dashboard (React + TypeScript)
- **Phase 3**: Bot detection (JA3/JA4 fingerprinting, JS challenges)
- **Phase 4**: Anti-scraping (CAPTCHA, content honeypots, dynamic obfuscation)

## License

MIT
