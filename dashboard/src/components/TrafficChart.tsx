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
                  <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="blockedGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#ef4444" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#2e2e2e" />
              <XAxis
                dataKey="time"
                stroke="#a0a0a0"
                fontSize={12}
                tickLine={false}
                axisLine={false}
              />
              <YAxis
                stroke="#a0a0a0"
                fontSize={12}
                tickLine={false}
                axisLine={false}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "#1c1c1c",
                  border: "1px solid #2e2e2e",
                  borderRadius: "0.5rem",
                  color: "#f5f5f5",
                }}
              />
              <Area
                type="monotone"
                dataKey="total"
                stroke="#3b82f6"
                fill="url(#totalGrad)"
                strokeWidth={2}
                name="Total"
              />
              <Area
                type="monotone"
                dataKey="blocked"
                stroke="#ef4444"
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
