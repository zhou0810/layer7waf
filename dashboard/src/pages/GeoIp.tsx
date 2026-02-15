import { Globe, ShieldBan, Search, MapPin } from "lucide-react";
import { PieChart, Pie, Cell, Tooltip } from "recharts";
import { useGeoIpStats, useStats } from "@/hooks/use-api";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

export function GeoIp() {
  const { data: geoStats, isLoading } = useGeoIpStats();
  const { data: stats } = useStats();

  const cleanRequests =
    stats && geoStats
      ? stats.total_requests - geoStats.geoip_blocked
      : 0;

  const pieData =
    geoStats && stats && stats.total_requests > 0
      ? [
          { name: "Allowed", value: cleanRequests > 0 ? cleanRequests : 0 },
          { name: "Geo-Blocked", value: geoStats.geoip_blocked },
        ]
      : [];

  const PIE_COLORS = ["#3b82f6", "#ef4444"];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">GeoIP Filtering</h2>
        <p className="text-muted-foreground text-sm">
          Country-based request filtering using MaxMind GeoLite2 database
        </p>
      </div>

      {/* Stats cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <Search className="h-4 w-4" />
              GeoIP Lookups
            </div>
            <p className="text-2xl font-bold">
              {isLoading
                ? "..."
                : (geoStats?.geoip_lookups.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <ShieldBan className="h-4 w-4" />
              Geo-Blocked
            </div>
            <p className="text-2xl font-bold">
              {isLoading
                ? "..."
                : (geoStats?.geoip_blocked.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <Globe className="h-4 w-4" />
              Status
            </div>
            <p className="text-2xl font-bold">
              {isLoading
                ? "..."
                : geoStats?.enabled
                  ? "Enabled"
                  : "Disabled"}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Charts and details row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">
              Allowed vs Geo-Blocked Traffic
            </CardTitle>
          </CardHeader>
          <CardContent>
            {pieData.length > 0 && pieData.some((d) => d.value > 0) ? (
              <div className="flex flex-col items-center">
                <PieChart width={260} height={260}>
                  <Pie
                    data={pieData}
                    cx={130}
                    cy={130}
                    innerRadius={60}
                    outerRadius={100}
                    dataKey="value"
                    strokeWidth={0}
                    isAnimationActive={false}
                  >
                    {pieData.map((_entry, index) => (
                      <Cell key={index} fill={PIE_COLORS[index]} />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "#1c1c1c",
                      border: "1px solid #2e2e2e",
                      borderRadius: "0.5rem",
                      color: "#f5f5f5",
                    }}
                  />
                </PieChart>
                <div className="flex gap-4 text-xs text-muted-foreground mt-2">
                  <span className="flex items-center gap-1">
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{ backgroundColor: PIE_COLORS[0] }}
                    />
                    Allowed
                  </span>
                  <span className="flex items-center gap-1">
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{ backgroundColor: PIE_COLORS[1] }}
                    />
                    Geo-Blocked
                  </span>
                </div>
              </div>
            ) : (
              <div className="h-[300px] flex items-center justify-center">
                <p className="text-muted-foreground text-sm">
                  No traffic data yet
                </p>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Configuration</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">
                  Total Lookups
                </span>
                <span className="font-mono text-sm">
                  {geoStats?.geoip_lookups.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">
                  Requests Blocked
                </span>
                <span className="font-mono text-sm text-red-400">
                  {geoStats?.geoip_blocked.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="py-2 border-b border-border">
                <div className="flex items-center gap-2 text-sm text-muted-foreground mb-2">
                  <MapPin className="h-4 w-4" />
                  Blocked Countries
                </div>
                <div className="flex flex-wrap gap-2">
                  {geoStats?.blocked_countries &&
                  geoStats.blocked_countries.length > 0 ? (
                    geoStats.blocked_countries.map((code) => (
                      <span
                        key={code}
                        className="px-2 py-1 bg-red-500/10 text-red-400 rounded text-xs font-mono"
                      >
                        {code}
                      </span>
                    ))
                  ) : (
                    <span className="text-xs text-muted-foreground">None</span>
                  )}
                </div>
              </div>
              <div className="py-2">
                <div className="flex items-center gap-2 text-sm text-muted-foreground mb-2">
                  <MapPin className="h-4 w-4" />
                  Allowed Countries
                </div>
                <div className="flex flex-wrap gap-2">
                  {geoStats?.allowed_countries &&
                  geoStats.allowed_countries.length > 0 ? (
                    geoStats.allowed_countries.map((code) => (
                      <span
                        key={code}
                        className="px-2 py-1 bg-emerald-500/10 text-emerald-400 rounded text-xs font-mono"
                      >
                        {code}
                      </span>
                    ))
                  ) : (
                    <span className="text-xs text-muted-foreground">None</span>
                  )}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
