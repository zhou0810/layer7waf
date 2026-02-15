import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from "recharts";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

interface DataPoint {
  time: string;
  total: number;
  blocked: number;
}

interface TrafficChartProps {
  data: DataPoint[];
}

export function TrafficChart({ data }: TrafficChartProps) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-base">Traffic Over Time</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="h-[300px]">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data} margin={{ top: 5, right: 10, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="totalGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="oklch(0.488 0.243 264.376)" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="oklch(0.488 0.243 264.376)" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="blockedGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="oklch(0.645 0.246 16.439)" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="oklch(0.645 0.246 16.439)" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.269 0 0)" />
              <XAxis
                dataKey="time"
                stroke="oklch(0.708 0 0)"
                fontSize={12}
                tickLine={false}
                axisLine={false}
              />
              <YAxis
                stroke="oklch(0.708 0 0)"
                fontSize={12}
                tickLine={false}
                axisLine={false}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "oklch(0.17 0 0)",
                  border: "1px solid oklch(0.269 0 0)",
                  borderRadius: "0.5rem",
                  color: "oklch(0.985 0 0)",
                }}
              />
              <Area
                type="monotone"
                dataKey="total"
                stroke="oklch(0.488 0.243 264.376)"
                fill="url(#totalGrad)"
                strokeWidth={2}
                name="Total"
              />
              <Area
                type="monotone"
                dataKey="blocked"
                stroke="oklch(0.645 0.246 16.439)"
                fill="url(#blockedGrad)"
                strokeWidth={2}
                name="Blocked"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}
