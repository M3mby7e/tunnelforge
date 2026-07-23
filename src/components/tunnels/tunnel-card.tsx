import {
  ArrowDown,
  ArrowRight,
  ArrowUp,
  Copy,
  Pencil,
  Play,
  Power,
  RefreshCw,
  ScrollText,
  Square,
  Trash2,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/tunnels/status-badge";
import { formatBytes, formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { TunnelConfig } from "@/types/generated/TunnelConfig";
import type { TunnelStatus } from "@/types/generated/TunnelStatus";
import type { TunnelState } from "@/types/generated/TunnelState";

const KIND_LABEL: Record<TunnelConfig["kind"], string> = {
  local: "Local",
  remote: "Remote",
  dynamic: "Dynamic",
};

const KIND_STYLE: Record<TunnelConfig["kind"], string> = {
  local: "bg-[color-mix(in_oklch,var(--brand)_16%,transparent)] text-brand",
  remote:
    "bg-[color-mix(in_oklch,var(--brand-accent)_18%,transparent)] text-brand-accent",
  dynamic:
    "bg-[color-mix(in_oklch,var(--success)_16%,transparent)] text-success",
};

interface TunnelCardProps {
  tunnel: TunnelConfig;
  status: TunnelStatus | undefined;
  onStart: () => void;
  onStop: () => void;
  onEdit: () => void;
  onLogs: () => void;
  onDuplicate: () => void;
  onToggleEnabled: () => void;
  onDelete: () => void;
}

export function TunnelCard({
  tunnel,
  status,
  onStart,
  onStop,
  onEdit,
  onLogs,
  onDuplicate,
  onToggleEnabled,
  onDelete,
}: TunnelCardProps) {
  const state: TunnelState = status?.state ?? "idle";
  const running =
    state === "connected" || state === "connecting" || state === "reconnecting";
  const listen = `${tunnel.listen.bindAddress}:${tunnel.listen.port}`;
  const target =
    tunnel.kind === "dynamic"
      ? "SOCKS5 · any destination"
      : tunnel.target
        ? `${tunnel.target.host}:${tunnel.target.port}`
        : "—";
  const stats = status?.stats;
  const showStats = state === "connected" && !!stats;

  return (
    <div
      className={cn(
        "tf-fade-in group bg-card flex flex-col gap-3 rounded-xl border p-4 shadow-sm transition-shadow hover:shadow-md",
        !tunnel.enabled && "opacity-60",
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={cn(
              "flex size-7 items-center justify-center rounded-md text-[11px] font-bold uppercase",
              KIND_STYLE[tunnel.kind],
            )}
            title={`${KIND_LABEL[tunnel.kind]} forwarding`}
          >
            {tunnel.kind.charAt(0)}
          </span>
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold">{tunnel.name}</p>
            <p className="text-muted-foreground truncate font-mono text-xs">
              {tunnel.ssh.username}@{tunnel.ssh.host}:{tunnel.ssh.port}
            </p>
          </div>
        </div>
        <StatusBadge state={state} />
      </div>

      <div className="bg-muted/50 flex items-center gap-2 rounded-lg px-2.5 py-1.5 font-mono text-xs">
        <span className="truncate">{listen}</span>
        <ArrowRight className="text-muted-foreground size-3.5 shrink-0" />
        <span className="text-muted-foreground truncate">{target}</span>
      </div>

      {showStats && (
        <div className="bg-muted/30 text-muted-foreground flex items-center gap-4 rounded-md px-2.5 py-1.5 text-xs">
          <span className="inline-flex items-center gap-1">
            <ArrowUp className="text-success size-3" />
            {formatBytes(stats.bytesUp)}
          </span>
          <span className="inline-flex items-center gap-1">
            <ArrowDown className="text-brand size-3" />
            {formatBytes(stats.bytesDown)}
          </span>
          <span>· {stats.activeConnections} conn</span>
          <span className="ml-auto">
            up {formatDuration(stats.uptimeSeconds)}
          </span>
        </div>
      )}

      {status?.message && state === "error" && (
        <p className="text-destructive truncate text-xs" title={status.message}>
          {status.message}
        </p>
      )}

      <div className="flex items-center justify-between">
        <div className="text-muted-foreground flex items-center gap-2 text-xs">
          <Badge variant="outline">{KIND_LABEL[tunnel.kind]}</Badge>
          {tunnel.reconnect.enabled && (
            <span
              className="inline-flex items-center gap-1"
              title="Auto-reconnect on"
            >
              <RefreshCw className="size-3" />
              auto
            </span>
          )}
          {!tunnel.enabled && <span>· disabled</span>}
        </div>
        <div className="flex items-center gap-0.5 opacity-70 transition-opacity group-hover:opacity-100">
          {running ? (
            <Button
              variant="ghost"
              size="icon-sm"
              title="Stop"
              onClick={onStop}
            >
              <Square />
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="icon-sm"
              title="Start"
              onClick={onStart}
              disabled={!tunnel.enabled}
            >
              <Play />
            </Button>
          )}
          <Button variant="ghost" size="icon-sm" title="Edit" onClick={onEdit}>
            <Pencil />
          </Button>
          <Button variant="ghost" size="icon-sm" title="Logs" onClick={onLogs}>
            <ScrollText />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            title="Duplicate"
            onClick={onDuplicate}
          >
            <Copy />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            title={tunnel.enabled ? "Disable" : "Enable"}
            onClick={onToggleEnabled}
          >
            <Power className={tunnel.enabled ? "text-success" : undefined} />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            title="Delete"
            onClick={onDelete}
          >
            <Trash2 />
          </Button>
        </div>
      </div>
    </div>
  );
}
