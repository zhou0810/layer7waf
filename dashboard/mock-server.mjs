// Mock API server for dashboard development
// Usage: node mock-server.mjs
// Serves realistic fake data on port 9090

import { createServer } from "node:http";

const startTime = Date.now();
let totalRequests = 147523;
let blockedRequests = 3891;
let rateLimitedRequests = 421;
let botsDetected = 234;
let challengesIssued = 512;
let challengesSolved = 389;
let customRules = [
  'SecRule ARGS "@contains <script>" "id:1001,phase:1,deny,status:403,msg:XSS attempt"',
  'SecRule REQUEST_URI "@rx /etc/passwd" "id:1002,phase:1,deny,status:403,msg:Path traversal"',
];

const config = {
  server: {
    listen: ["0.0.0.0:8080"],
    tls: null,
    admin: { listen: "127.0.0.1:9090", dashboard: true },
  },
  upstreams: [
    {
      name: "backend",
      servers: [
        { addr: "127.0.0.1:3000", weight: 1 },
        { addr: "127.0.0.1:3001", weight: 1 },
      ],
      health_check: { interval_secs: 10, path: "/health" },
    },
  ],
  routes: [
    {
      host: null,
      path_prefix: "/",
      upstream: "backend",
      waf: { enabled: true, mode: "block" },
      rate_limit: { rps: 100, burst: 200, algorithm: "token_bucket" },
    },
    {
      host: "api.example.com",
      path_prefix: "/v1",
      upstream: "backend",
      waf: { enabled: true, mode: "detect" },
      rate_limit: null,
    },
  ],
  waf: {
    rules: ["rules/owasp-crs/*.conf", "rules/custom/*.conf"],
    request_body_limit: 13107200,
    audit_log: { enabled: true, path: "/var/log/layer7waf/audit.log" },
  },
  rate_limit: { enabled: true, default_rps: 100, default_burst: 200 },
  ip_reputation: { blocklist: "rules/blocklist.txt", allowlist: null },
  bot_detection: {
    enabled: true,
    mode: "challenge",
    js_challenge: {
      enabled: true,
      difficulty: 16,
      ttl_secs: 3600,
      secret: "mock-secret-key",
    },
    score_threshold: 0.7,
    known_bots_allowlist: ["Googlebot", "Bingbot"],
  },
};

const actions = ["allowed", "blocked", "rate_limited", "allowed", "allowed", "allowed", "allowed", "allowed"];
const methods = ["GET", "GET", "GET", "POST", "PUT", "DELETE", "GET", "GET"];
const uris = [
  "/",
  "/api/users",
  "/api/products?q=search",
  "/api/login",
  "/api/orders",
  "/static/app.js",
  "/api/health",
  '/api/users?id=1 OR 1=1',
  "/images/logo.png",
  "/.env",
  "/api/upload",
  "/admin/config",
];
const ips = [
  "192.168.1.42",
  "10.0.0.15",
  "172.16.0.100",
  "203.0.113.55",
  "198.51.100.12",
  "192.0.2.88",
  "10.10.10.10",
  "172.217.14.99",
];
const ruleIds = [null, null, null, null, "941100", "942100", "949110", null, "932100", null];

function generateLogs(limit, offset, ipFilter, ruleIdFilter) {
  // Generate a pool of ~200 log entries
  const all = [];
  const baseTime = Date.now() - 3600000;
  for (let i = 0; i < 200; i++) {
    const action = actions[i % actions.length];
    const ruleId = action === "blocked" ? ruleIds[i % ruleIds.length] || "941100" : ruleIds[i % ruleIds.length];
    const status = action === "blocked" ? 403 : action === "rate_limited" ? 429 : 200;
    all.push({
      id: `log-${String(i).padStart(4, "0")}`,
      timestamp: new Date(baseTime + i * 18000).toISOString(),
      client_ip: ips[i % ips.length],
      method: methods[i % methods.length],
      uri: uris[i % uris.length],
      rule_id: ruleId,
      action,
      status,
    });
  }

  let filtered = all;
  if (ipFilter) filtered = filtered.filter((e) => e.client_ip === ipFilter);
  if (ruleIdFilter) filtered = filtered.filter((e) => e.rule_id === ruleIdFilter);

  return {
    total: filtered.length,
    offset,
    limit,
    entries: filtered.slice(offset, offset + limit),
  };
}

function generateMetrics() {
  const uptimeSecs = Math.floor((Date.now() - startTime) / 1000);
  return `# HELP waf_requests_total Total number of requests processed
# TYPE waf_requests_total counter
waf_requests_total ${totalRequests}
# HELP waf_requests_blocked Total number of requests blocked by WAF rules
# TYPE waf_requests_blocked counter
waf_requests_blocked ${blockedRequests}
# HELP waf_rate_limited_total Total number of requests rate-limited
# TYPE waf_rate_limited_total counter
waf_rate_limited_total ${rateLimitedRequests}
# HELP waf_request_duration_seconds Request processing duration in seconds
# TYPE waf_request_duration_seconds histogram
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.001"} 52341
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.005"} 98234
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.01"} 121456
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.025"} 135678
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.05"} 140123
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.1"} 143456
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.25"} 145678
waf_request_duration_seconds_bucket{method="GET",status="200",le="0.5"} 146789
waf_request_duration_seconds_bucket{method="GET",status="200",le="1.0"} 147234
waf_request_duration_seconds_bucket{method="GET",status="200",le="5.0"} 147490
waf_request_duration_seconds_bucket{method="GET",status="200",le="+Inf"} ${totalRequests}
waf_request_duration_seconds_sum{method="GET",status="200"} ${(uptimeSecs * 0.8).toFixed(3)}
waf_request_duration_seconds_count{method="GET",status="200"} ${totalRequests}
# HELP waf_rule_hits_total Number of times each WAF rule was triggered
# TYPE waf_rule_hits_total counter
waf_rule_hits_total{rule_id="941100"} 1523
waf_rule_hits_total{rule_id="942100"} 891
waf_rule_hits_total{rule_id="949110"} 445
waf_rule_hits_total{rule_id="932100"} 312
`;
}

// Simulate traffic growth
setInterval(() => {
  const inc = Math.floor(Math.random() * 30) + 5;
  totalRequests += inc;
  if (Math.random() < 0.08) blockedRequests += Math.floor(Math.random() * 3) + 1;
  if (Math.random() < 0.03) rateLimitedRequests += 1;
  if (Math.random() < 0.05) {
    botsDetected += Math.floor(Math.random() * 2) + 1;
    challengesIssued += Math.floor(Math.random() * 3) + 1;
    challengesSolved += Math.floor(Math.random() * 2);
  }
}, 1000);

const server = createServer((req, res) => {
  const url = new URL(req.url, `http://${req.headers.host}`);
  const path = url.pathname;

  // CORS headers
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type");

  if (req.method === "OPTIONS") {
    res.writeHead(204);
    res.end();
    return;
  }

  const json = (data, status = 200) => {
    res.writeHead(status, { "Content-Type": "application/json" });
    res.end(JSON.stringify(data));
  };

  const readBody = () =>
    new Promise((resolve) => {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => resolve(body));
    });

  // Routes
  if (path === "/api/health" && req.method === "GET") {
    const uptimeSecs = Math.floor((Date.now() - startTime) / 1000);
    json({ status: "healthy", uptime_secs: uptimeSecs, version: "0.1.0" });
  } else if (path === "/api/stats" && req.method === "GET") {
    const uptimeSecs = Math.floor((Date.now() - startTime) / 1000);
    const rps = uptimeSecs > 0 ? totalRequests / uptimeSecs : 0;
    json({
      total_requests: totalRequests,
      blocked_requests: blockedRequests,
      rate_limited_requests: rateLimitedRequests,
      uptime_secs: uptimeSecs,
      requests_per_second: Math.round(rps * 10) / 10,
    });
  } else if (path === "/api/metrics" && req.method === "GET") {
    res.writeHead(200, { "Content-Type": "text/plain; version=0.0.4; charset=utf-8" });
    res.end(generateMetrics());
  } else if (path === "/api/config" && req.method === "GET") {
    json(config);
  } else if (path === "/api/config" && req.method === "PUT") {
    readBody().then((body) => {
      try {
        const newConfig = JSON.parse(body);
        Object.assign(config, newConfig);
        json({ status: "updated" });
      } catch {
        json({ status: "error", message: "Invalid JSON" }, 400);
      }
    });
  } else if (path === "/api/rules" && req.method === "GET") {
    json({
      rule_files: config.waf.rules,
      custom_rules: customRules.map((rule, i) => ({ id: i, rule })),
    });
  } else if (path === "/api/rules" && req.method === "POST") {
    readBody().then((body) => {
      const { rule } = JSON.parse(body);
      if (!rule || !rule.trim()) {
        json({ status: "error", message: "rule must not be empty" }, 400);
        return;
      }
      const id = customRules.length;
      customRules.push(rule);
      json({ status: "created", id, rule }, 201);
    });
  } else if (path === "/api/rules/test" && req.method === "POST") {
    readBody().then((body) => {
      const data = JSON.parse(body);
      const matched = data.rule.includes("@contains") && data.request.uri.includes("attack");
      json({
        matched,
        rule: data.rule,
        request: { method: data.request.method, uri: data.request.uri },
        message: matched ? "Rule matched the request" : "Rule did not match the request",
      });
    });
  } else if (path.startsWith("/api/rules/") && req.method === "DELETE") {
    const id = parseInt(path.split("/").pop(), 10);
    if (id >= 0 && id < customRules.length) {
      const removed = customRules.splice(id, 1)[0];
      json({ status: "deleted", id, rule: removed });
    } else {
      json({ status: "error", message: `rule with id ${id} not found` }, 404);
    }
  } else if (path === "/api/bot-stats" && req.method === "GET") {
    const passRate = challengesIssued > 0 ? challengesSolved / challengesIssued : 0;
    json({
      bots_detected: botsDetected,
      challenges_issued: challengesIssued,
      challenges_solved: challengesSolved,
      challenge_pass_rate: Math.round(passRate * 1000) / 1000,
    });
  } else if (path === "/api/logs" && req.method === "GET") {
    const limit = parseInt(url.searchParams.get("limit") || "100", 10);
    const offset = parseInt(url.searchParams.get("offset") || "0", 10);
    const ip = url.searchParams.get("ip") || undefined;
    const ruleId = url.searchParams.get("rule_id") || undefined;
    json(generateLogs(limit, offset, ip, ruleId));
  } else {
    json({ status: "error", message: "Not found" }, 404);
  }
});

server.listen(9090, () => {
  console.log("Mock API server running on http://localhost:9090");
  console.log("Dashboard dev server should proxy /api/* here");
});
