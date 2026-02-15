import { useState } from "react";
import { useLogs } from "@/hooks/use-api";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { ErrorAlert } from "@/components/ErrorAlert";
import { Skeleton } from "@/components/Skeleton";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { ChevronLeft, ChevronRight, RefreshCw } from "lucide-react";

const PAGE_SIZE = 20;

export function Logs() {
  const [ipFilter, setIpFilter] = useState("");
  const [ruleIdFilter, setRuleIdFilter] = useState("");
  const [page, setPage] = useState(0);
  const [autoRefresh, setAutoRefresh] = useState(false);

  const { data, isLoading, error, refetch } = useLogs({
    limit: PAGE_SIZE,
    offset: page * PAGE_SIZE,
    ip: ipFilter || undefined,
    rule_id: ruleIdFilter || undefined,
    autoRefresh,
  });

  const totalPages = data ? Math.ceil(data.total / PAGE_SIZE) : 0;

  function actionBadgeVariant(action: string): "default" | "destructive" | "secondary" | "outline" {
    if (action === "blocked") return "destructive";
    if (action === "rate_limited") return "outline";
    return "secondary";
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Audit Logs</h2>
        <p className="text-muted-foreground text-sm">Request inspection and WAF event log</p>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-end gap-4">
        <div className="space-y-1">
          <Label>Client IP</Label>
          <Input
            placeholder="Filter by IP..."
            value={ipFilter}
            onChange={(e) => {
              setIpFilter(e.target.value);
              setPage(0);
            }}
            className="w-48"
          />
        </div>
        <div className="space-y-1">
          <Label>Rule ID</Label>
          <Input
            placeholder="Filter by rule ID..."
            value={ruleIdFilter}
            onChange={(e) => {
              setRuleIdFilter(e.target.value);
              setPage(0);
            }}
            className="w-48"
          />
        </div>
        <div className="flex items-center gap-2">
          <Switch checked={autoRefresh} onCheckedChange={setAutoRefresh} />
          <Label className="text-sm">Auto-refresh</Label>
        </div>
        <Button variant="outline" size="sm" onClick={() => refetch()}>
          <RefreshCw className="h-4 w-4 mr-1" />
          Refresh
        </Button>
        {data && (
          <span className="text-sm text-muted-foreground ml-auto">
            {data.total} total entries
          </span>
        )}
      </div>

      {error && (
        <ErrorAlert message={error.message} onRetry={() => refetch()} />
      )}

      {/* Table */}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Timestamp</TableHead>
            <TableHead>Client IP</TableHead>
            <TableHead>Method</TableHead>
            <TableHead>URI</TableHead>
            <TableHead>Rule ID</TableHead>
            <TableHead>Action</TableHead>
            <TableHead>Status</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {isLoading && !data ? (
            Array.from({ length: 5 }).map((_, i) => (
              <TableRow key={i}>
                {Array.from({ length: 7 }).map((_, j) => (
                  <TableCell key={j}><Skeleton className="h-4 w-full" /></TableCell>
                ))}
              </TableRow>
            ))
          ) : data && data.entries.length > 0 ? (
            data.entries.map((entry) => (
              <TableRow key={entry.id}>
                <TableCell className="font-mono text-xs whitespace-nowrap">
                  {entry.timestamp}
                </TableCell>
                <TableCell className="font-mono text-xs">{entry.client_ip}</TableCell>
                <TableCell>
                  <Badge variant="outline">{entry.method}</Badge>
                </TableCell>
                <TableCell className="font-mono text-xs max-w-[300px] truncate">
                  {entry.uri}
                </TableCell>
                <TableCell className="text-xs">
                  {entry.rule_id ?? "—"}
                </TableCell>
                <TableCell>
                  <Badge variant={actionBadgeVariant(entry.action)}>
                    {entry.action}
                  </Badge>
                </TableCell>
                <TableCell>{entry.status}</TableCell>
              </TableRow>
            ))
          ) : (
            <TableRow>
              <TableCell colSpan={7} className="text-center text-muted-foreground py-8">
                No log entries found
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={page === 0}
            onClick={() => setPage((p) => p - 1)}
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <span className="text-sm text-muted-foreground">
            Page {page + 1} of {totalPages}
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages - 1}
            onClick={() => setPage((p) => p + 1)}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      )}
    </div>
  );
}
