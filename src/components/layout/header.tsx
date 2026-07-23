import { Cable, Play, Plus, Search, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ThemeToggle } from "@/components/theme-toggle";

interface HeaderProps {
  query: string;
  onQueryChange: (value: string) => void;
  onStartAll: () => void;
  onStopAll: () => void;
  onAdd: () => void;
}

export function Header({
  query,
  onQueryChange,
  onStartAll,
  onStopAll,
  onAdd,
}: HeaderProps) {
  return (
    <header className="bg-card/60 flex items-center gap-3 border-b px-4 py-2.5 backdrop-blur">
      <div className="flex items-center gap-2.5">
        <div className="from-brand to-brand-accent text-brand-foreground flex size-8 items-center justify-center rounded-lg bg-gradient-to-br shadow-sm">
          <Cable className="size-4.5" />
        </div>
        <div className="leading-tight">
          <p className="text-sm font-semibold">Tunnelium</p>
          <p className="text-muted-foreground text-[10px] tracking-wide uppercase">
            SSH tunnel manager
          </p>
        </div>
      </div>

      <div className="relative ml-2 hidden max-w-xs flex-1 items-center sm:flex">
        <Search className="text-muted-foreground pointer-events-none absolute left-2.5 size-4" />
        <input
          value={query}
          onChange={(e) => onQueryChange(e.currentTarget.value)}
          placeholder="Search tunnels…"
          className="bg-background placeholder:text-muted-foreground focus-visible:ring-ring/60 h-9 w-full rounded-md border pr-3 pl-8 text-sm outline-none focus-visible:ring-2"
        />
      </div>

      <div className="ml-auto flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={onStartAll}>
          <Play />
          <span className="hidden md:inline">Start all</span>
        </Button>
        <Button variant="outline" size="sm" onClick={onStopAll}>
          <Square />
          <span className="hidden md:inline">Stop all</span>
        </Button>
        <Button variant="brand" size="sm" onClick={onAdd}>
          <Plus />
          <span className="hidden md:inline">Add tunnel</span>
        </Button>
        <div className="bg-border mx-1 h-6 w-px" />
        <ThemeToggle />
      </div>
    </header>
  );
}
