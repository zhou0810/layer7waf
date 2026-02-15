import { useMetrics } from "@/hooks/use-api";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { ErrorAlert } from "@/components/ErrorAlert";
import { Skeleton } from "@/components/Skeleton";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

interface ParsedCounter {
  name: string;
  help: string;
  value: number;
}

interface HistogramBucket {
  le: string;
  count: number;
}

interface ParsedHistogram {
  name: string;
  buckets: HistogramBucket[];
  sum: number;
  count: number;
}

function parsePrometheusText(text: string): {
  counters: ParsedCounter[];
  histograms: ParsedHistogram[];
} {
  const counters: ParsedCounter[] = [];
  const histograms: ParsedHistogram[] = [];
  const lines = text.split("\n");

  const helpMap = new Map<string, string>();
  const typeMap = new Map<string, string>();

  for (const line of lines) {
    if (line.startsWith("# HELP ")) {
      const rest = line.slice(7);
      const spaceIdx = rest.indexOf(" ");
      if (spaceIdx > 0) {
        helpMap.set(rest.slice(0, spaceIdx), rest.slice(spaceIdx + 1));
      }
    } else if (line.startsWith("# TYPE ")) {
      const rest = line.slice(7);
      const spaceIdx = rest.indexOf(" ");
      if (spaceIdx > 0) {
        typeMap.set(rest.slice(0, spaceIdx), rest.slice(spaceIdx + 1));
      }
    }
  }

  const histogramBuckets = new Map<string, HistogramBucket[]>();
  const histogramSums = new Map<string, number>();
  const histogramCounts = new Map<string, number>();

  for (const line of lines) {
    if (line.startsWith("#") || line.trim() === "") continue;

    // Parse histogram bucket lines: name_bucket{le="0.01"} 5
    const bucketMatch = line.match(/^(\w+)_bucket\{.*le="([^"]+)".*\}\s+(\d+(?:\.\d+)?)/);
    if (bucketMatch) {
      const baseName = bucketMatch[1];
      const le = bucketMatch[2];
      const count = parseFloat(bucketMatch[3]);
      if (!histogramBuckets.has(baseName)) histogramBuckets.set(baseName, []);
      histogramBuckets.get(baseName)!.push({ le, count });
      continue;
    }

    const sumMatch = line.match(/^(\w+)_sum\s+(\d+(?:\.\d+)?)/);
    if (sumMatch) {
      histogramSums.set(sumMatch[1], parseFloat(sumMatch[2]));
      continue;
    }

    const countMatch = line.match(/^(\w+)_count\s+(\d+(?:\.\d+)?)/);
    if (countMatch) {
      histogramCounts.set(countMatch[1], parseFloat(countMatch[2]));
      continue;
    }

    // Parse counter lines: name 123 or name{labels} 123
    const counterMatch = line.match(/^(\w+?)(?:\{[^}]*\})?\s+(\d+(?:\.\d+)?)/);
    if (counterMatch) {
      const name = counterMatch[1];
      const value = parseFloat(counterMatch[2]);
      const type = typeMap.get(name);
      if (type === "counter" || type === "gauge") {
        // Avoid duplicates
        if (!counters.find((c) => c.name === name)) {
          counters.push({
            name,
            help: helpMap.get(name) ?? "",
            value,
          });
        }
      }
    }
  }

  for (const [name, buckets] of histogramBuckets) {
    histograms.push({
      name,
      buckets,
      sum: histogramSums.get(name) ?? 0,
      count: histogramCounts.get(name) ?? 0,
    });
  }

  return { counters, histograms };
}

export function Metrics() {
  const { data: rawMetrics, isLoading, error, refetch } = useMetrics();

  const parsed = rawMetrics ? parsePrometheusText(rawMetrics) : null;

  // Build histogram chart data (show differential counts per bucket)
  const histogramChartData = parsed?.histograms[0]?.buckets
    .filter((b) => b.le !== "+Inf")
    .map((b, i, arr) => ({
      bucket: `<=${b.le}`,
      count: i === 0 ? b.count : b.count - arr[i - 1].count,
    })) ?? [];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Metrics</h2>
        <p className="text-muted-foreground text-sm">
          Prometheus metrics (auto-refresh every 10s)
        </p>
      </div>

      {error && (
        <ErrorAlert message={error.message} onRetry={() => refetch()} />
      )}

      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <Card key={i}>
              <CardContent className="p-6">
                <Skeleton className="h-4 w-32 mb-3" />
                <Skeleton className="h-10 w-24" />
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <Tabs defaultValue="cards">
          <TabsList>
            <TabsTrigger value="cards">Overview</TabsTrigger>
            <TabsTrigger value="histogram">Latency Histogram</TabsTrigger>
            <TabsTrigger value="raw">Raw</TabsTrigger>
          </TabsList>

          <TabsContent value="cards">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {parsed?.counters.map((c) => (
                <Card key={c.name}>
                  <CardHeader className="pb-1">
                    <CardTitle className="text-sm font-mono">{c.name}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-3xl font-bold">{c.value.toLocaleString()}</p>
                    {c.help && (
                      <p className="text-xs text-muted-foreground mt-1">{c.help}</p>
                    )}
                  </CardContent>
                </Card>
              ))}
              {parsed?.histograms.map((h) => (
                <Card key={h.name}>
                  <CardHeader className="pb-1">
                    <CardTitle className="text-sm font-mono">{h.name}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-xl font-bold">
                      avg {h.count > 0 ? `${((h.sum / h.count) * 1000).toFixed(1)}ms` : "—"}
                    </p>
                    <p className="text-xs text-muted-foreground mt-1">
                      {h.count} samples, sum {h.sum.toFixed(3)}s
                    </p>
                  </CardContent>
                </Card>
              ))}
            </div>
          </TabsContent>

          <TabsContent value="histogram">
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Request Duration Distribution</CardTitle>
              </CardHeader>
              <CardContent>
                {histogramChartData.length > 0 ? (
                  <div className="h-[400px]">
                    <ResponsiveContainer width="100%" height="100%">
                      <BarChart data={histogramChartData} margin={{ top: 5, right: 10, left: 0, bottom: 0 }}>
                        <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.269 0 0)" />
                        <XAxis
                          dataKey="bucket"
                          stroke="oklch(0.708 0 0)"
                          fontSize={12}
                          tickLine={false}
                        />
                        <YAxis
                          stroke="oklch(0.708 0 0)"
                          fontSize={12}
                          tickLine={false}
                        />
                        <Tooltip
                          contentStyle={{
                            backgroundColor: "oklch(0.17 0 0)",
                            border: "1px solid oklch(0.269 0 0)",
                            borderRadius: "0.5rem",
                            color: "oklch(0.985 0 0)",
                          }}
                        />
                        <Bar dataKey="count" fill="oklch(0.488 0.243 264.376)" radius={[4, 4, 0, 0]} />
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                ) : (
                  <p className="text-muted-foreground text-sm">No histogram data available</p>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="raw">
            <Card>
              <CardContent className="pt-6">
                <pre className="text-xs font-mono whitespace-pre-wrap bg-muted rounded-md p-4 max-h-[600px] overflow-auto">
                  {rawMetrics || "No metrics data"}
                </pre>
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}
