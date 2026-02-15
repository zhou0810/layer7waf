import { AlertTriangle } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

interface ErrorAlertProps {
  message: string;
  onRetry?: () => void;
}

export function ErrorAlert({ message, onRetry }: ErrorAlertProps) {
  return (
    <Card className="border-destructive/50">
      <CardContent className="p-4 flex items-center gap-3">
        <AlertTriangle className="h-5 w-5 text-destructive-foreground shrink-0" />
        <div className="flex-1">
          <p className="text-sm font-medium">Failed to load data</p>
          <p className="text-xs text-muted-foreground mt-0.5">{message}</p>
        </div>
        {onRetry && (
          <Button variant="outline" size="sm" onClick={onRetry}>
            Retry
          </Button>
        )}
      </CardContent>
    </Card>
  );
}
