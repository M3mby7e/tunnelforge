import { Boxes, FolderClosed, Plug, Settings, Unplug } from "lucide-react";
import { cn } from "@/lib/utils";

export interface SidebarGroup {
  label: string;
  count: number;
}

interface SidebarProps {
  groups: SidebarGroup[];
  active: string;
  onSelect: (label: string) => void;
  connectedCount: number;
  stoppedCount: number;
  onOpenSettings: () => void;
}

export function Sidebar({
  groups,
  active,
  onSelect,
  connectedCount,
  stoppedCount,
  onOpenSettings,
}: SidebarProps) {
  return (
    <aside className="bg-sidebar flex w-56 shrink-0 flex-col border-r">
      <div className="flex-1 overflow-auto p-3">
        <Section title="Groups">
          <NavItem
            icon={<Boxes className="size-4" />}
            label="All tunnels"
            count={groups.reduce((n, g) => n + g.count, 0)}
            active={active === "All tunnels"}
            onClick={() => onSelect("All tunnels")}
          />
          {groups.map((g) => (
            <NavItem
              key={g.label}
              icon={<FolderClosed className="size-4" />}
              label={g.label}
              count={g.count}
              active={active === g.label}
              onClick={() => onSelect(g.label)}
            />
          ))}
        </Section>

        <Section title="Status">
          <NavItem
            icon={<Plug className="text-success size-4" />}
            label="Connected"
            count={connectedCount}
          />
          <NavItem
            icon={<Unplug className="text-muted-foreground size-4" />}
            label="Stopped"
            count={stoppedCount}
          />
        </Section>
      </div>

      <div className="border-t p-3">
        <NavItem
          icon={<Settings className="size-4" />}
          label="Settings"
          onClick={onOpenSettings}
        />
      </div>
    </aside>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="mb-4">
      <p className="text-muted-foreground px-2 pb-1.5 text-[11px] font-semibold tracking-wide uppercase">
        {title}
      </p>
      <nav className="space-y-0.5">{children}</nav>
    </div>
  );
}

interface NavItemProps {
  icon: React.ReactNode;
  label: string;
  count?: number;
  active?: boolean;
  onClick?: () => void;
}

function NavItem({ icon, label, count, active, onClick }: NavItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
        active
          ? "bg-accent text-accent-foreground font-medium"
          : "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
      )}
    >
      {icon}
      <span className="flex-1 truncate text-left">{label}</span>
      {count !== undefined && (
        <span className="text-muted-foreground text-xs tabular-nums">
          {count}
        </span>
      )}
    </button>
  );
}
