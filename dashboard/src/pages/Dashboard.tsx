import { useRef, useEffect, useCallback } from "react";
import { Activity, ShieldAlert, Clock, Gauge } from "lucide-react";
import { PieChart, Pie, Cell, Tooltip } from "recharts";
import { useHealth, useStats } from "@/hooks/use-api";
import { StatsCard } from "@/components/StatsCard";
import { StatusBadge } from "@/components/StatusBadge";
import { TrafficChart } from "@/components/TrafficChart";
import { Skeleton } from "@/components/Skeleton";
import { ErrorAlert } from "@/components/ErrorAlert";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

interface TrafficPoint {
  time: string;
  total: number;
  blocked: number;
}

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m ${s}s`;
  return `${m}m ${s}s`;
}

export function Dashboard() {
  const { data: health, isLoading: healthLoading, error: healthError, refetch: refetchHealth } = useHealth();
  const { data: stats, isLoading: statsLoading, error: statsError, refetch: refetchStats } = useStats();

  const trafficRef = useRef<TrafficPoint[]>([]);
  const prevTotalRef = useRef<number | null>(null);
  const prevBlockedRef = useRef<number | null>(null);

  const updateTraffic = useCallback(() => {
    if (!stats) return;
    const now = new Date();
    const time = `${now.getHours().toString().padStart(2, "0")}:${now.getMinutes().toString().padStart(2, "0")}:${now.getSeconds().toString().padStart(2, "0")}`;

    const totalDelta =
      prevTotalRef.current !== null
        ? stats.total_requests - prevTotalRef.current
        : 0;
    const blockedDelta =
      prevBlockedRef.current !== null
        ? stats.blocked_requests - prevBlockedRef.current
        : 0;

    prevTotalRef.current = stats.total_requests;
    prevBlockedRef.current = stats.blocked_requests;

    trafficRef.current = [
      ...trafficRef.current.slice(-59),
      { time, total: totalDelta, blocked: blockedDelta },
    ];
  }, [stats]);

  useEffect(() => {
    updateTraffic();
  }, [updateTraffic]);

  const blockRate = stats && stats.total_requests > 0
    ? ((stats.blocked_requests / stats.total_requests) * 100).toFixed(1)
    : "0.0";

  const pieData = stats
    ? [
        { name: "Allowed", value: stats.total_requests - stats.blocked_requests },
        { name: "Blocked", value: stats.blocked_requests },
      ]
    : [];

  const PIE_COLORS = ["#3b82f6", "#ef4444"];

  const hasError = healthError || statsError;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Dashboard</h2>
          <p className="text-muted-foreground text-sm">WAF overview and traffic monitoring</p>
        </div>
        {healthLoading ? (
          <Skeleton className="h-6 w-20" />
        ) : health ? (
          <StatusBadge status={health.status} />
        ) : null}
      </div>

      {hasError && (
        <ErrorAlert
          message={(healthError || statsError)?.message ?? "Connection failed"}
          onRetry={() => { refetchHealth(); refetchStats(); }}
        />
      )}

      {/* Stats cards */}
      {statsLoading && !stats ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Card key={i}>
              <CardContent className="p-6">
                <Skeleton className="h-4 w-24 mb-3" />
                <Skeleton className="h-8 w-20" />
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <StatsCard
            title="Total Requests"
            value={stats?.total_requests.toLocaleString() ?? "—"}
            icon={Activity}
          />
          <StatsCard
            title="Blocked Requests"
            value={stats?.blocked_requests.toLocaleString() ?? "—"}
            description={`${blockRate}% block rate`}
            icon={ShieldAlert}
          />
          <StatsCard
            title="Requests/sec"
            value={stats?.requests_per_second.toFixed(1) ?? "—"}
            icon={Gauge}
          />
          <StatsCard
            title="Uptime"
            value={stats ? formatUptime(stats.uptime_secs) : "—"}
            description={health ? `v${health.version}` : undefined}
            icon={Clock}
          />
        </div>
      )}

      {/* Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2">
          <TrafficChart data={trafficRef.current} />
        </div>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Block Rate</CardTitle>
          </CardHeader>
          <CardContent>
            {stats && stats.total_requests > 0 ? (
              <div className="flex flex-col items-center">
                <PieChart width={220} height={220}>
                  <Pie
                    data={pieData}
                    cx={110}
                    cy={110}
                    innerRadius={60}
                    outerRadius={90}
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
                <p className="text-2xl font-bold mt-2">{blockRate}%</p>
                <div className="flex gap-4 text-xs text-muted-foreground mt-2">
                  <span className="flex items-center gap-1">
                    <span className="h-2 w-2 rounded-full" style={{ backgroundColor: PIE_COLORS[0] }} />
                    Allowed
                  </span>
                  <span className="flex items-center gap-1">
                    <span className="h-2 w-2 rounded-full" style={{ backgroundColor: PIE_COLORS[1] }} />
                    Blocked
                  </span>
                </div>
              </div>
            ) : statsLoading ? (
              <div className="h-[300px] flex items-center justify-center">
                <Skeleton className="h-[180px] w-[180px] rounded-full" />
              </div>
            ) : (
              <div className="h-[300px] flex items-center justify-center">
                <p className="text-muted-foreground text-sm">No traffic data yet</p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
