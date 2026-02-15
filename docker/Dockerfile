# Stage 1: Build Go shared library
FROM golang:1.22-bookworm AS go-builder

WORKDIR /build/coraza
COPY crates/coraza/go/ ./
RUN go mod download
RUN CGO_ENABLED=1 go build -buildmode=c-shared -o /build/libcoraza_bridge.so .

# Stage 2: Build dashboard
FROM node:20-bookworm-slim AS dashboard-builder

WORKDIR /build/dashboard
COPY dashboard/package.json dashboard/package-lock.json* ./
RUN npm ci
COPY dashboard/ ./
RUN npm run build

# Stage 3: Build Rust binary
FROM rust:1.77-bookworm AS rust-builder

# Install Go for the coraza build.rs (it re-compiles the Go code)
COPY --from=go-builder /usr/local/go /usr/local/go
ENV PATH="/usr/local/go/bin:${PATH}"

WORKDIR /build

# Copy workspace
COPY Cargo.toml Cargo.lock* ./
COPY crates/ crates/

# Pre-populate the Go module cache so build.rs doesn't re-download
COPY --from=go-builder /go/pkg/mod /root/go/pkg/mod

# Build release binary
RUN cargo build --release --bin layer7waf

# Stage 4: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create WAF directories
RUN mkdir -p /etc/layer7waf/certs \
             /etc/layer7waf/rules/custom \
             /var/log/layer7waf

# Copy binary
COPY --from=rust-builder /build/target/release/layer7waf /usr/local/bin/layer7waf

# Copy Go shared library
COPY --from=go-builder /build/libcoraza_bridge.so /usr/local/lib/
RUN ldconfig

# Copy dashboard
COPY --from=dashboard-builder /build/dashboard/dist /usr/share/layer7waf/dashboard
ENV DASHBOARD_DIR=/usr/share/layer7waf/dashboard

# Copy default config
COPY config/layer7waf.yaml /etc/layer7waf/layer7waf.yaml

# Copy OWASP CRS rules (if present)
COPY rules/ /etc/layer7waf/rules/

EXPOSE 8080 443 9090

ENTRYPOINT ["layer7waf"]
CMD ["/etc/layer7waf/layer7waf.yaml"]
