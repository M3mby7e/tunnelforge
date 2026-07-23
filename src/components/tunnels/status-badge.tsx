import type { TunnelState } from "@/types/generated/TunnelState";
import { cn } from "@/lib/utils";

const CONFIG: Record<
  TunnelState,
  { label: string; dot: string; text: string; pulse?: boolean }
> = {
  connected: {
    label: "Connected",
    dot: "bg-success",
    text: "text-success",
    pulse: true,
  },
  connecting: {
    label: "Connecting",
    dot: "bg-warning",
    text: "text-warning",
    pulse: true,
  },
  reconnecting: {
    label: "Reconnecting",
    dot: "bg-warning",
    text: "text-warning",
    pulse: true,
  },
  stopping: {
    label: "Stopping",
    dot: "bg-muted-foreground/60",
    text: "text-muted-foreground",
    pulse: true,
  },
  idle: {
    label: "Stopped",
    dot: "bg-muted-foreground/50",
    text: "text-muted-foreground",
  },
  error: { label: "Error", dot: "bg-destructive", text: "text-destructive" },
};

interface StatusBadgeProps {
  state: TunnelState;
}

export function StatusBadge({ state }: StatusBadgeProps) {
  const { label, dot, text, pulse } = CONFIG[state];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 text-xs font-medium",
        text,
      )}
    >
      <span className="relative flex size-2">
        {pulse && (
          <span
            className={cn(
              "absolute inline-flex h-full w-full animate-ping rounded-full opacity-60",
              dot,
            )}
          />
        )}
        <span className={cn("relative inline-flex size-2 rounded-full", dot)} />
      </span>
      {label}
    </span>
  );
}
