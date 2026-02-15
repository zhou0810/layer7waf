import { Bot, ShieldCheck, ShieldQuestion, Percent } from "lucide-react";
import { PieChart, Pie, Cell, Tooltip } from "recharts";
import { useBotStats, useStats } from "@/hooks/use-api";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

export function BotDetection() {
  const { data: botStats, isLoading: botLoading } = useBotStats();
  const { data: stats } = useStats();

  const passRate = botStats
    ? (botStats.challenge_pass_rate * 100).toFixed(1)
    : "0.0";

  const humanRequests = stats && botStats
    ? stats.total_requests - botStats.bots_detected
    : 0;

  const pieData = botStats && stats && stats.total_requests > 0
    ? [
        { name: "Human", value: humanRequests > 0 ? humanRequests : 0 },
        { name: "Bot", value: botStats.bots_detected },
      ]
    : [];

  const PIE_COLORS = ["#3b82f6", "#f59e0b"];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Bot Detection</h2>
        <p className="text-muted-foreground text-sm">
          Bot traffic analysis and JS challenge statistics
        </p>
      </div>

      {/* Stats cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <Bot className="h-4 w-4" />
              Bots Detected
            </div>
            <p className="text-2xl font-bold">
              {botLoading ? "..." : (botStats?.bots_detected.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <ShieldQuestion className="h-4 w-4" />
              Challenges Issued
            </div>
            <p className="text-2xl font-bold">
              {botLoading ? "..." : (botStats?.challenges_issued.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <ShieldCheck className="h-4 w-4" />
              Challenges Solved
            </div>
            <p className="text-2xl font-bold">
              {botLoading ? "..." : (botStats?.challenges_solved.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <Percent className="h-4 w-4" />
              Challenge Pass Rate
            </div>
            <p className="text-2xl font-bold">{passRate}%</p>
          </CardContent>
        </Card>
      </div>

      {/* Bot vs Human pie chart */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Bot vs Human Traffic</CardTitle>
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
                    Human
                  </span>
                  <span className="flex items-center gap-1">
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{ backgroundColor: PIE_COLORS[1] }}
                    />
                    Bot
                  </span>
                </div>
              </div>
            ) : (
              <div className="h-[300px] flex items-center justify-center">
                <p className="text-muted-foreground text-sm">No traffic data yet</p>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Detection Summary</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">Total Requests</span>
                <span className="font-mono text-sm">
                  {stats?.total_requests.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">Bots Detected</span>
                <span className="font-mono text-sm text-amber-400">
                  {botStats?.bots_detected.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">Challenges Issued</span>
                <span className="font-mono text-sm">
                  {botStats?.challenges_issued.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">Challenges Solved</span>
                <span className="font-mono text-sm text-emerald-400">
                  {botStats?.challenges_solved.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2">
                <span className="text-sm text-muted-foreground">Bot Traffic Rate</span>
                <span className="font-mono text-sm">
                  {stats && botStats && stats.total_requests > 0
                    ? ((botStats.bots_detected / stats.total_requests) * 100).toFixed(2)
                    : "0.00"}
                  %
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
