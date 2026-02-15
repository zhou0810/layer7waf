import { ShieldBan, Bug, ShieldCheck, ShieldQuestion, Eye } from "lucide-react";
import { PieChart, Pie, Cell, Tooltip } from "recharts";
import { useScrapingStats, useStats } from "@/hooks/use-api";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

export function AntiScraping() {
  const { data: scrapingStats, isLoading } = useScrapingStats();
  const { data: stats } = useStats();

  const passRate = scrapingStats
    ? (scrapingStats.captcha_pass_rate * 100).toFixed(1)
    : "0.0";

  const cleanRequests = stats && scrapingStats
    ? stats.total_requests - scrapingStats.scrapers_blocked
    : 0;

  const pieData = scrapingStats && stats && stats.total_requests > 0
    ? [
        { name: "Clean", value: cleanRequests > 0 ? cleanRequests : 0 },
        { name: "Scrapers", value: scrapingStats.scrapers_blocked },
      ]
    : [];

  const PIE_COLORS = ["#3b82f6", "#ef4444"];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Anti-Scraping</h2>
        <p className="text-muted-foreground text-sm">
          Scraper detection, honeypot traps, CAPTCHA challenges, and response obfuscation
        </p>
      </div>

      {/* Stats cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <ShieldBan className="h-4 w-4" />
              Scrapers Blocked
            </div>
            <p className="text-2xl font-bold">
              {isLoading ? "..." : (scrapingStats?.scrapers_blocked.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <Bug className="h-4 w-4" />
              Traps Triggered
            </div>
            <p className="text-2xl font-bold">
              {isLoading ? "..." : (scrapingStats?.traps_triggered.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <ShieldQuestion className="h-4 w-4" />
              CAPTCHAs Issued
            </div>
            <p className="text-2xl font-bold">
              {isLoading ? "..." : (scrapingStats?.captchas_issued.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <ShieldCheck className="h-4 w-4" />
              CAPTCHAs Solved
            </div>
            <p className="text-2xl font-bold">
              {isLoading ? "..." : (scrapingStats?.captchas_solved.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
              <Eye className="h-4 w-4" />
              Obfuscated
            </div>
            <p className="text-2xl font-bold">
              {isLoading ? "..." : (scrapingStats?.responses_obfuscated.toLocaleString() ?? "0")}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Charts row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Clean vs Scraper Traffic</CardTitle>
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
                    Clean
                  </span>
                  <span className="flex items-center gap-1">
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{ backgroundColor: PIE_COLORS[1] }}
                    />
                    Scrapers
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
                <span className="text-sm text-muted-foreground">Scrapers Blocked</span>
                <span className="font-mono text-sm text-red-400">
                  {scrapingStats?.scrapers_blocked.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">Traps Triggered</span>
                <span className="font-mono text-sm text-amber-400">
                  {scrapingStats?.traps_triggered.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">CAPTCHAs Issued</span>
                <span className="font-mono text-sm">
                  {scrapingStats?.captchas_issued.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">CAPTCHAs Solved</span>
                <span className="font-mono text-sm text-emerald-400">
                  {scrapingStats?.captchas_solved.toLocaleString() ?? "0"}
                </span>
              </div>
              <div className="flex justify-between items-center py-2 border-b border-border">
                <span className="text-sm text-muted-foreground">CAPTCHA Pass Rate</span>
                <span className="font-mono text-sm">{passRate}%</span>
              </div>
              <div className="flex justify-between items-center py-2">
                <span className="text-sm text-muted-foreground">Responses Obfuscated</span>
                <span className="font-mono text-sm">
                  {scrapingStats?.responses_obfuscated.toLocaleString() ?? "0"}
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
