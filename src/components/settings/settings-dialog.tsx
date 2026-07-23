import { useEffect, useState } from "react";
import { Download, Upload } from "lucide-react";
import { Button, buttonVariants } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { Modal } from "@/components/ui/modal";
import { ThemeToggle } from "@/components/theme-toggle";
import { getStartOnBoot, isTauri, setStartOnBoot } from "@/api/backend";
import { useStore } from "@/state/store";
import type { AppConfig } from "@/types/generated/AppConfig";

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

/** Remove keychain references before exporting a shareable config. */
function stripSecrets(config: AppConfig): AppConfig {
  return {
    ...config,
    tunnels: config.tunnels.map((t) => ({
      ...t,
      auth:
        t.auth.kind === "password"
          ? { kind: "password", secretRef: "" }
          : t.auth.kind === "privateKey"
            ? { ...t.auth, passphraseRef: null }
            : t.auth.kind === "keyboardInteractive"
              ? { kind: "keyboardInteractive", secretRef: null }
              : t.auth,
      proxy: t.proxy ? { ...t.proxy, authRef: null } : null,
    })),
  };
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const config = useStore((s) => s.config);
  const updateSettings = useStore((s) => s.updateSettings);
  const [startOnBoot, setStartOnBootState] = useState(false);

  useEffect(() => {
    if (open && isTauri) void getStartOnBoot().then(setStartOnBootState);
  }, [open]);

  if (!config) return null;
  const prefs = config.notifications;

  const setNotif = (key: keyof AppConfig["notifications"], value: boolean) =>
    void updateSettings({ notifications: { ...prefs, [key]: value } });

  async function toggleBoot(on: boolean) {
    setStartOnBootState(on);
    await setStartOnBoot(on);
    await updateSettings({ startOnBoot: on });
  }

  function exportConfig() {
    const json = JSON.stringify(stripSecrets(config!), null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "tunnelium-config.json";
    a.click();
    URL.revokeObjectURL(url);
  }

  function importConfig(file: File) {
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const parsed = JSON.parse(String(reader.result)) as AppConfig;
        const existing = config!.tunnels;
        const merged = [...existing, ...(parsed.tunnels ?? [])];
        void updateSettings({ tunnels: merged });
      } catch {
        // ignore invalid files
      }
    };
    reader.readAsText(file);
  }

  return (
    <Modal
      open={open}
      title="Settings"
      onClose={onClose}
      footer={
        <Button variant="brand" onClick={onClose}>
          Done
        </Button>
      }
    >
      <div className="space-y-5">
        <Group title="Appearance">
          <Row label="Theme">
            <ThemeToggle />
          </Row>
        </Group>

        <Group title="Behavior">
          <Checkbox
            checked={config.minimizeToTray}
            onChange={(v) => void updateSettings({ minimizeToTray: v })}
            label="Minimize to the system tray when closed"
          />
          <Checkbox
            checked={startOnBoot}
            onChange={(v) => void toggleBoot(v)}
            label="Start Tunnelium when I log in"
            hint={isTauri ? undefined : "Available in the desktop app"}
          />
        </Group>

        <Group title="Notifications">
          <Checkbox
            checked={prefs.onConnect}
            onChange={(v) => setNotif("onConnect", v)}
            label="When a tunnel connects"
          />
          <Checkbox
            checked={prefs.onDisconnect}
            onChange={(v) => setNotif("onDisconnect", v)}
            label="When a tunnel disconnects"
          />
          <Checkbox
            checked={prefs.onError}
            onChange={(v) => setNotif("onError", v)}
            label="When a tunnel errors"
          />
        </Group>

        <Group title="Configuration">
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={exportConfig}>
              <Download />
              Export (no secrets)
            </Button>
            <label
              className={cn(
                buttonVariants({ variant: "outline", size: "sm" }),
                "cursor-pointer",
              )}
            >
              <Upload />
              Import
              <input
                type="file"
                accept="application/json"
                className="hidden"
                onChange={(e) => {
                  const file = e.currentTarget.files?.[0];
                  if (file) importConfig(file);
                  e.currentTarget.value = "";
                }}
              />
            </label>
          </div>
          <p className="text-xs text-muted-foreground">
            Export omits all secrets; re-enter passwords/passphrases after import.
          </p>
        </Group>
      </div>
    </Modal>
  );
}

function Group({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {title}
      </p>
      {children}
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm">{label}</span>
      {children}
    </div>
  );
}
