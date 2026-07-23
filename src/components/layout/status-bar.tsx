interface StatusBarProps {
  total: number;
  connected: number;
}

export function StatusBar({ total, connected }: StatusBarProps) {
  return (
    <footer className="bg-card/60 text-muted-foreground flex items-center justify-between border-t px-4 py-1.5 text-xs">
      <span>
        {total} tunnel{total === 1 ? "" : "s"} ·{" "}
        <span className="text-success">{connected} connected</span>
      </span>
      <span className="inline-flex items-center gap-1.5">
        <span className="bg-success size-1.5 rounded-full" />
        Ready
      </span>
    </footer>
  );
}
