# Layer 7 WAF

A high-performance Layer 7 Web Application Firewall built with [Pingora](https://github.com/cloudflare/pingora) (Rust proxy framework) and [Coraza](https://github.com/corazawaf/coraza) (OWASP CRS-compatible WAF engine).

## Architecture

```
                    ┌──────────────────────────────────────────────────┐
                    │                 Layer 7 WAF                        │
  Client ──HTTP──►  │                                                    │
                    │  ┌──────────┐  ┌────────┐  ┌────────────────────┐ │
                    │  │ Pingora  │  │ Coraza │  │ Admin API (Axum)   │ │
                    │  │ Proxy    ├─►│ Bridge ├─►│ + React Dashboard  │ │
                    │  │ Engine   │  │ (FFI)  │  │ + REST API         │ │
                    │  └────┬─────┘  └────────┘  └────────────────────┘ │
                    │       │                                            │
                    │  ┌────┴─────┐  ┌──────────┐  ┌───────────────┐   │
                    │  │ Rate     │  │ IP       │  │ Bot Detection │   │
                    │  │ Limiter  │  │ Reputa-  │  │ (Fingerprint, │   │
                    │  │          │  │ tion     │  │  JS Challenge)│   │
                    │  └──────────┘  └──────────┘  └───────────────┘   │
                    └──────────────────┬────────────────────────────────┘
                                       │
                                       ▼
                                  Upstream Servers
```

### Request Lifecycle

```
1. request_filter()   → IP check → Rate limit → Bot detection → Coraza WAF
2. upstream_peer()    → Select upstream (weighted round-robin)
3. response_filter()  → Coraza response headers/body check
4. logging()          → Structured JSON log, Prometheus metrics
```

## Features

- **WAF Engine**: Coraza WAF via Go FFI bridge with OWASP CRS compatibility
- **Rate Limiting**: Token bucket and sliding window algorithms with per-IP tracking
- **IP Reputation**: CIDR prefix trie for fast blocklist/allowlist lookups with hot-reload
- **Bot Detection**: HTTP fingerprinting, User-Agent classification, JS proof-of-work challenges
- **Reverse Proxy**: Weighted round-robin upstream selection via Pingora
- **Admin REST API**: Health, metrics, config, rules management, audit logs, bot stats
- **Web Dashboard**: React/TypeScript UI for monitoring, configuration, and bot analytics
- **Observability**: Prometheus metrics, structured JSON logging

## Project Structure

```
layer7waf/
├── crates/
│   ├── proxy/          # Main binary - Pingora ProxyHttp pipeline
│   ├── coraza/         # Coraza WAF FFI bridge (Go → C shared lib → Rust)
│   ├── rate-limit/     # Token bucket & sliding window rate limiters
│   ├── ip-reputation/  # CIDR prefix trie for IP blocklist/allowlist
│   ├── bot-detect/     # Bot detection: fingerprinting, JS challenges, scoring
│   ├── admin/          # Axum REST API server
│   └── common/         # Shared config structs and error types
├── dashboard/          # React/TypeScript web dashboard
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

bot_detection:
  enabled: true
  mode: challenge            # block | challenge | detect
  score_threshold: 0.7       # 0.0-1.0, requests scoring above are flagged
  js_challenge:
    enabled: true
    difficulty: 16           # leading zero bits for proof-of-work
    ttl_secs: 3600           # challenge cookie validity
    secret: "your-hmac-key"  # HMAC signing key (random default)
  known_bots_allowlist:
    - Googlebot
    - Bingbot
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
| `/api/bot-stats` | GET | Bot detection statistics |

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

## Bot Detection

The bot detection module sits between rate limiting and WAF checks in the request pipeline, combining multiple signals to identify automated traffic.

### Detection Signals

- **HTTP Fingerprinting** — SHA-256 hash of ordered header names, User-Agent family extraction, Accept header combination hash. Different tools produce distinct header orderings that serve as fingerprints.
- **User-Agent Classification** — Requests are classified as `KnownGoodBot` (Googlebot, Bingbot, etc.), `KnownBadBot` (curl, wget, python-requests, scrapy), `Suspicious` (generic bot/crawler/spider patterns), or `LikelyHuman` (standard browser UAs).
- **JS Proof-of-Work Challenge** — Suspected bots receive an HTML page with embedded JavaScript that computes SHA-256 hashes until finding one with the required leading zero bits. On success, an HMAC-signed cookie is set and the browser redirects to the original URL. Real browsers solve this transparently; headless scripts and CLI tools cannot.

### Scoring

Each request receives a composite bot score from 0.0 (human) to 1.0 (bot):

| Signal | Score Impact |
|---|---|
| Known bad bot UA (curl, scrapy, etc.) | +0.9 |
| Suspicious UA (generic bot patterns) | +0.5 |
| Missing standard Accept header | +0.2 |
| Valid JS challenge cookie | -0.8 |
| Known good bot (Googlebot, etc.) | 0.0 (always allowed) |

### Modes

- **`block`** — Requests exceeding the score threshold are rejected with 403.
- **`challenge`** — Requests exceeding the threshold receive a JS challenge page. If the challenge is already solved (valid cookie), the request proceeds.
- **`detect`** — All requests proceed, but bot scores are recorded in metrics for monitoring.

```bash
# View bot detection stats
curl http://localhost:9090/api/bot-stats

# Returns: { bots_detected, challenges_issued, challenges_solved, challenge_pass_rate }
```

## Dashboard

The React/TypeScript dashboard is served from the admin API when `server.admin.dashboard` is enabled.

```bash
# Development with mock API
cd dashboard
npm install
npm run dev:mock    # Starts mock server on :9090 + Vite on :5173

# Production build
npm run build       # Output in dashboard/dist/
```

Pages: Dashboard (traffic overview), Audit Logs, WAF Rules, Bot Detection (analytics + pie chart), Configuration (structured editor), Metrics (Prometheus).

## Docker

```bash
cd docker
docker compose up
```

This starts the WAF proxy on port 8080 and the admin API on port 9090, with an nginx upstream backend.

## Testing

```bash
# Run all unit tests (56 tests)
cargo test --workspace

# Run bot detection tests only
cargo test -p layer7waf-bot-detect

# Build dashboard
cd dashboard && npm run build

# Run E2E tests (requires running docker-compose stack)
./tests/e2e/test_waf.sh
```

## Roadmap

- **Phase 1** &#10003;: WAF core, rate limiting, IP reputation, admin API
- **Phase 2** &#10003;: Web dashboard (React + TypeScript)
- **Phase 3** &#10003;: Bot detection (HTTP fingerprinting, JS challenges, scoring)
- **Phase 4**: Anti-scraping (CAPTCHA, content honeypots, dynamic obfuscation)
- **Future**: TLS fingerprinting (JA3/JA4) when Pingora exposes Client Hello data

## License

MIT
