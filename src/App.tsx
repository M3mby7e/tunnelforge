import { useEffect, useMemo, useState } from "react";
import { Cable, Plus } from "lucide-react";
import { Header } from "@/components/layout/header";
import { Sidebar, type SidebarGroup } from "@/components/layout/sidebar";
import { StatusBar } from "@/components/layout/status-bar";
import { TunnelCard } from "@/components/tunnels/tunnel-card";
import { TunnelForm } from "@/components/tunnels/tunnel-form";
import { SettingsDialog } from "@/components/settings/settings-dialog";
import { LogViewer } from "@/components/logs/log-viewer";
import { Button } from "@/components/ui/button";
import { useStore } from "@/state/store";
import type { TunnelConfig } from "@/types/generated/TunnelConfig";
import type { TunnelState } from "@/types/generated/TunnelState";

const ALL = "All tunnels";
const RUNNING: TunnelState[] = ["connected", "connecting", "reconnecting"];

function App() {
  const ready = useStore((s) => s.ready);
  const config = useStore((s) => s.config);
  const statuses = useStore((s) => s.statuses);
  const init = useStore((s) => s.init);
  const start = useStore((s) => s.start);
  const stop = useStore((s) => s.stop);
  const startAll = useStore((s) => s.startAll);
  const stopAll = useStore((s) => s.stopAll);
  const deleteTunnel = useStore((s) => s.deleteTunnel);
  const duplicateTunnel = useStore((s) => s.duplicateTunnel);
  const setEnabled = useStore((s) => s.setEnabled);

  const [query, setQuery] = useState("");
  const [activeGroup, setActiveGroup] = useState(ALL);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<TunnelConfig | null>(null);
  const [formSeq, setFormSeq] = useState(0);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logTunnelId, setLogTunnelId] = useState<string | null>(null);

  useEffect(() => {
    void init();
  }, [init]);

  const tunnels = useMemo(() => config?.tunnels ?? [], [config]);

  const stateOf = (id: string): TunnelState => statuses[id]?.state ?? "idle";

  const groups = useMemo<SidebarGroup[]>(() => {
    const counts = new Map<string, number>();
    for (const t of tunnels) {
      const label = t.group ?? "Ungrouped";
      counts.set(label, (counts.get(label) ?? 0) + 1);
    }
    return [...counts.entries()].map(([label, count]) => ({ label, count }));
  }, [tunnels]);

  const connectedCount = tunnels.filter(
    (t) => stateOf(t.id) === "connected",
  ).length;
  const stoppedCount = tunnels.filter(
    (t) => !RUNNING.includes(stateOf(t.id)),
  ).length;

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return tunnels.filter((t) => {
      const label = t.group ?? "Ungrouped";
      const inGroup = activeGroup === ALL || label === activeGroup;
      const matches =
        q === "" ||
        t.name.toLowerCase().includes(q) ||
        t.ssh.host.toLowerCase().includes(q) ||
        `${t.listen.bindAddress}:${t.listen.port}`.includes(q);
      return inGroup && matches;
    });
  }, [tunnels, query, activeGroup]);

  const openCreate = () => {
    setEditing(null);
    setFormSeq((n) => n + 1);
    setFormOpen(true);
  };
  const openEdit = (tunnel: TunnelConfig) => {
    setEditing(tunnel);
    setFormSeq((n) => n + 1);
    setFormOpen(true);
  };

  return (
    <div className="bg-background text-foreground flex h-full flex-col">
      <Header
        query={query}
        onQueryChange={setQuery}
        onStartAll={() => void startAll()}
        onStopAll={() => void stopAll()}
        onAdd={openCreate}
      />
      <div className="flex min-h-0 flex-1">
        <Sidebar
          groups={groups}
          active={activeGroup}
          onSelect={setActiveGroup}
          connectedCount={connectedCount}
          stoppedCount={stoppedCount}
          onOpenSettings={() => setSettingsOpen(true)}
        />
        <main className="min-w-0 flex-1 overflow-auto">
          <div className="flex items-center justify-between px-6 py-4">
            <div>
              <h1 className="text-base font-semibold">{activeGroup}</h1>
              <p className="text-muted-foreground text-xs">
                {visible.length} tunnel{visible.length === 1 ? "" : "s"}
              </p>
            </div>
            <Button variant="brand" size="sm" onClick={openCreate}>
              <Plus />
              New tunnel
            </Button>
          </div>

          {!ready ? null : visible.length === 0 ? (
            <EmptyState onCreate={openCreate} empty={tunnels.length === 0} />
          ) : (
            <div className="grid grid-cols-1 gap-3 px-6 pb-6 lg:grid-cols-2 2xl:grid-cols-3">
              {visible.map((t) => (
                <TunnelCard
                  key={t.id}
                  tunnel={t}
                  status={statuses[t.id]}
                  onStart={() => void start(t.id)}
                  onStop={() => void stop(t.id)}
                  onEdit={() => openEdit(t)}
                  onLogs={() => setLogTunnelId(t.id)}
                  onDuplicate={() => void duplicateTunnel(t.id)}
                  onToggleEnabled={() => void setEnabled(t.id, !t.enabled)}
                  onDelete={() => void deleteTunnel(t.id)}
                />
              ))}
            </div>
          )}
        </main>
      </div>
      <StatusBar total={tunnels.length} connected={connectedCount} />

      <TunnelForm
        key={formSeq}
        open={formOpen}
        initial={editing}
        onClose={() => setFormOpen(false)}
      />
      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
      <LogViewer
        open={logTunnelId !== null}
        tunnelId={logTunnelId}
        onClose={() => setLogTunnelId(null)}
      />
    </div>
  );
}

interface EmptyStateProps {
  onCreate: () => void;
  empty: boolean;
}

function EmptyState({ onCreate, empty }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 py-24 text-center">
      <div className="bg-muted flex size-14 items-center justify-center rounded-full">
        <Cable className="text-muted-foreground size-7" />
      </div>
      <h2 className="text-lg font-semibold">
        {empty ? "No tunnels yet" : "No tunnels here"}
      </h2>
      <p className="text-muted-foreground max-w-sm text-sm">
        {empty
          ? "Create your first SSH tunnel to forward a port through a server."
          : "Nothing matches this view. Clear the search or pick another group."}{" "}
        New to tunneling? Read <code>docs/TUNNELING_GUIDE.md</code>.
      </p>
      {empty && (
        <Button variant="brand" size="sm" onClick={onCreate}>
          <Plus />
          New tunnel
        </Button>
      )}
    </div>
  );
}

export default App;
