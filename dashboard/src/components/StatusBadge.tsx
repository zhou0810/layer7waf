import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface StatusBadgeProps {
  status: string;
  className?: string;
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const isHealthy = status === "healthy";
  return (
    <Badge
      className={cn(
        isHealthy
          ? "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
          : "bg-red-500/20 text-red-400 border-red-500/30",
        className
      )}
    >
      <span
        className={cn(
          "mr-1.5 h-2 w-2 rounded-full inline-block",
          isHealthy ? "bg-emerald-400" : "bg-red-400"
        )}
      />
      {status}
    </Badge>
  );
}
