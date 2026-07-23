import { Modal } from "@/components/ui/modal";
import { useStore } from "@/state/store";
import { cn } from "@/lib/utils";
import type { LogLine } from "@/types/generated/LogLine";

interface LogViewerProps {
  open: boolean;
  tunnelId: string | null;
  onClose: () => void;
}

const LEVEL_CLASS: Record<LogLine["level"], string> = {
  info: "text-foreground",
  warn: "text-warning",
  error: "text-destructive",
};

export function LogViewer({ open, tunnelId, onClose }: LogViewerProps) {
  const logs = useStore((s) => (tunnelId ? s.logs[tunnelId] : undefined)) ?? [];
  const name = useStore(
    (s) => s.config?.tunnels.find((t) => t.id === tunnelId)?.name,
  );

  return (
    <Modal open={open} title={`Logs — ${name ?? "tunnel"}`} onClose={onClose}>
      <div className="max-h-[60vh] min-h-40 overflow-auto rounded-md bg-muted/40 p-3 font-mono text-xs leading-relaxed">
        {logs.length === 0 ? (
          <p className="text-muted-foreground">
            No log lines yet. Start the tunnel to see connection activity.
          </p>
        ) : (
          logs.map((line, index) => (
            <div key={index} className={cn("whitespace-pre-wrap", LEVEL_CLASS[line.level])}>
              <span className="text-muted-foreground">
                {new Date(line.ts).toLocaleTimeString()}
              </span>{" "}
              {line.line}
            </div>
          ))
        )}
      </div>
    </Modal>
  );
}
