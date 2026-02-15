// TypeScript interfaces mirroring Rust config structs from crates/common/src/config.rs

export interface AppConfig {
  server: ServerConfig;
  upstreams: UpstreamConfig[];
  routes: RouteConfig[];
  waf: WafConfig;
  rate_limit: RateLimitConfig;
  ip_reputation: IpReputationConfig;
  bot_detection: BotDetectionConfig;
}

export interface ServerConfig {
  listen: string[];
  tls?: TlsConfig | null;
  admin: AdminConfig;
}

export interface TlsConfig {
  cert: string;
  key: string;
}

export interface AdminConfig {
  listen: string;
  dashboard: boolean;
}

export interface UpstreamConfig {
  name: string;
  servers: UpstreamServer[];
  health_check?: HealthCheckConfig | null;
}

export interface UpstreamServer {
  addr: string;
  weight: number;
}

export interface HealthCheckConfig {
  interval_secs: number;
  path: string;
}

export interface RouteConfig {
  host?: string | null;
  path_prefix: string;
  upstream: string;
  waf: RouteWafConfig;
  rate_limit?: RouteRateLimitConfig | null;
}

export interface RouteWafConfig {
  enabled: boolean;
  mode: "block" | "detect" | "off";
}

export interface RouteRateLimitConfig {
  rps: number;
  burst: number;
  algorithm: "token_bucket" | "sliding_window";
}

export interface WafConfig {
  rules: string[];
  request_body_limit: number;
  audit_log: AuditLogConfig;
}

export interface AuditLogConfig {
  enabled: boolean;
  path: string;
}

export interface RateLimitConfig {
  enabled: boolean;
  default_rps: number;
  default_burst: number;
}

export interface IpReputationConfig {
  blocklist?: string | null;
  allowlist?: string | null;
}

export interface BotDetectionConfig {
  enabled: boolean;
  mode: "block" | "challenge" | "detect";
  js_challenge: JsChallengeConfig;
  score_threshold: number;
  known_bots_allowlist: string[];
}

export interface JsChallengeConfig {
  enabled: boolean;
  difficulty: number;
  ttl_secs: number;
  secret: string;
}

export interface BotStatsResponse {
  bots_detected: number;
  challenges_issued: number;
  challenges_solved: number;
  challenge_pass_rate: number;
}

export interface ScrapingStatsResponse {
  scrapers_blocked: number;
  traps_triggered: number;
  captchas_issued: number;
  captchas_solved: number;
  responses_obfuscated: number;
  captcha_pass_rate: number;
}

export interface GeoIpStatsResponse {
  geoip_blocked: number;
  geoip_lookups: number;
  enabled: boolean;
  blocked_countries: string[];
  allowed_countries: string[];
}

// API response types

export interface HealthResponse {
  status: string;
  uptime_secs: number;
  version: string;
}

export interface StatsResponse {
  total_requests: number;
  blocked_requests: number;
  rate_limited_requests: number;
  uptime_secs: number;
  requests_per_second: number;
}

export interface AuditLogEntry {
  id: string;
  timestamp: string;
  client_ip: string;
  method: string;
  uri: string;
  rule_id: string | null;
  action: string;
  status: number;
}

export interface LogsResponse {
  total: number;
  offset: number;
  limit: number;
  entries: AuditLogEntry[];
}

export interface RulesResponse {
  rule_files: string[];
  custom_rules: { id: number; rule: string }[];
}

export interface TestRuleRequest {
  rule: string;
  request: {
    method: string;
    uri: string;
    headers: Record<string, string>;
    body?: string;
  };
}

export interface TestRuleResponse {
  matched: boolean;
  rule: string;
  request: { method: string; uri: string };
  message: string;
}

// API client functions

const BASE = "/api";

async function fetchJSON<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`API error ${res.status}: ${text}`);
  }
  return res.json();
}

export const api = {
  getHealth: () => fetchJSON<HealthResponse>(`${BASE}/health`),

  getStats: () => fetchJSON<StatsResponse>(`${BASE}/stats`),

  getMetrics: async (): Promise<string> => {
    const res = await fetch(`${BASE}/metrics`);
    if (!res.ok) throw new Error(`API error ${res.status}`);
    return res.text();
  },

  getConfig: () => fetchJSON<AppConfig>(`${BASE}/config`),

  updateConfig: (config: AppConfig) =>
    fetchJSON<{ status: string }>(`${BASE}/config`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    }),

  getRules: () => fetchJSON<RulesResponse>(`${BASE}/rules`),

  addRule: (rule: string) =>
    fetchJSON<{ status: string; id: number; rule: string }>(`${BASE}/rules`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ rule }),
    }),

  deleteRule: (id: number) =>
    fetchJSON<{ status: string }>(`${BASE}/rules/${id}`, {
      method: "DELETE",
    }),

  testRule: (data: TestRuleRequest) =>
    fetchJSON<TestRuleResponse>(`${BASE}/rules/test`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    }),

  getBotStats: () => fetchJSON<BotStatsResponse>(`${BASE}/bot-stats`),

  getScrapingStats: () => fetchJSON<ScrapingStatsResponse>(`${BASE}/scraping-stats`),

  getGeoIpStats: () => fetchJSON<GeoIpStatsResponse>(`${BASE}/geoip-stats`),

  getLogs: (params?: { limit?: number; offset?: number; ip?: string; rule_id?: string }) => {
    const searchParams = new URLSearchParams();
    if (params?.limit) searchParams.set("limit", String(params.limit));
    if (params?.offset) searchParams.set("offset", String(params.offset));
    if (params?.ip) searchParams.set("ip", params.ip);
    if (params?.rule_id) searchParams.set("rule_id", params.rule_id);
    const qs = searchParams.toString();
    return fetchJSON<LogsResponse>(`${BASE}/logs${qs ? `?${qs}` : ""}`);
  },
};
