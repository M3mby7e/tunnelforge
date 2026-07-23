import { create } from "zustand";
import type { AppConfig } from "@/types/generated/AppConfig";
import type { LogLine } from "@/types/generated/LogLine";
import type { TunnelConfig } from "@/types/generated/TunnelConfig";
import type { TunnelStatus } from "@/types/generated/TunnelStatus";
import {
  isTauri,
  loadConfig,
  notify,
  onLog,
  onStatus,
  saveConfig,
  startAllTunnels,
  startTunnel,
  stopAllTunnels,
  stopTunnel,
} from "@/api/backend";
import type { TunnelState } from "@/types/generated/TunnelState";
import { demoConfig, demoStatuses } from "@/state/demo";
import { defaultAppConfig } from "@/lib/config-defaults";

const MAX_LOG_LINES = 500;

interface StoreState {
  config: AppConfig | null;
  statuses: Record<string, TunnelStatus>;
  logs: Record<string, LogLine[]>;
  ready: boolean;

  init: () => Promise<void>;
  updateSettings: (patch: Partial<AppConfig>) => Promise<void>;
  saveTunnel: (tunnel: TunnelConfig) => Promise<void>;
  duplicateTunnel: (id: string) => Promise<void>;
  setEnabled: (id: string, enabled: boolean) => Promise<void>;
  deleteTunnel: (id: string) => Promise<void>;
  start: (id: string) => Promise<void>;
  stop: (id: string) => Promise<void>;
  startAll: () => Promise<void>;
  stopAll: () => Promise<void>;
}

function upsert(tunnels: TunnelConfig[], tunnel: TunnelConfig): TunnelConfig[] {
  const index = tunnels.findIndex((t) => t.id === tunnel.id);
  if (index === -1) return [...tunnels, tunnel];
  return tunnels.map((t) => (t.id === tunnel.id ? tunnel : t));
}

export const useStore = create<StoreState>((set, get) => ({
  config: null,
  statuses: {},
  logs: {},
  ready: false,

  init: async () => {
    if (!isTauri) {
      set({ config: demoConfig(), statuses: demoStatuses(), ready: true });
      return;
    }
    const config = await loadConfig();
    set({ config, ready: true });

    await onStatus((status) => {
      const prev = get().statuses[status.id]?.state;
      set((state) => ({
        statuses: { ...state.statuses, [status.id]: status },
      }));
      void notifyTransition(get().config, prev, status);
    });
    await onLog((line) =>
      set((state) => {
        const existing = state.logs[line.id] ?? [];
        const next = [...existing, line].slice(-MAX_LOG_LINES);
        return { logs: { ...state.logs, [line.id]: next } };
      }),
    );

    // Auto-start flagged tunnels once event listeners are attached.
    for (const tunnel of config.tunnels) {
      if (tunnel.autoStart && tunnel.enabled) {
        void get().start(tunnel.id);
      }
    }
  },

  updateSettings: async (patch) => {
    const config = get().config ?? defaultAppConfig();
    const next: AppConfig = { ...config, ...patch };
    if (isTauri) await saveConfig(next);
    set({ config: next });
  },

  saveTunnel: async (tunnel) => {
    const config = get().config ?? defaultAppConfig();
    const next: AppConfig = {
      ...config,
      tunnels: upsert(config.tunnels, tunnel),
    };
    if (isTauri) await saveConfig(next); // throws on validation error
    set({ config: next });
  },

  duplicateTunnel: async (id) => {
    const source = get().config?.tunnels.find((t) => t.id === id);
    if (!source) return;
    const now = new Date().toISOString();
    const copy: TunnelConfig = {
      ...source,
      id: crypto.randomUUID(),
      name: `${source.name} (copy)`,
      createdAt: now,
      updatedAt: now,
    };
    await get().saveTunnel(copy);
  },

  setEnabled: async (id, enabled) => {
    const source = get().config?.tunnels.find((t) => t.id === id);
    if (!source) return;
    if (!enabled) await get().stop(id);
    await get().saveTunnel({
      ...source,
      enabled,
      updatedAt: new Date().toISOString(),
    });
  },

  deleteTunnel: async (id) => {
    const config = get().config;
    if (!config) return;
    if (isTauri) await stopTunnel(id).catch(() => undefined);
    const next: AppConfig = {
      ...config,
      tunnels: config.tunnels.filter((t) => t.id !== id),
    };
    if (isTauri) await saveConfig(next);
    set((state) => {
      const statuses = { ...state.statuses };
      const logs = { ...state.logs };
      delete statuses[id];
      delete logs[id];
      return { config: next, statuses, logs };
    });
  },

  start: async (id) => {
    if (isTauri) {
      await startTunnel(id);
    } else {
      set((state) => ({
        statuses: { ...state.statuses, [id]: fakeStatus(id, "connected") },
      }));
    }
  },

  stop: async (id) => {
    if (isTauri) {
      await stopTunnel(id);
    } else {
      set((state) => ({
        statuses: { ...state.statuses, [id]: fakeStatus(id, "idle") },
      }));
    }
  },

  startAll: async () => {
    if (isTauri) {
      await startAllTunnels();
      return;
    }
    const config = get().config;
    if (!config) return;
    set((state) => {
      const statuses = { ...state.statuses };
      for (const t of config.tunnels) {
        if (t.enabled) statuses[t.id] = fakeStatus(t.id, "connected");
      }
      return { statuses };
    });
  },

  stopAll: async () => {
    if (isTauri) {
      await stopAllTunnels();
      return;
    }
    set((state) => {
      const statuses = { ...state.statuses };
      for (const id of Object.keys(statuses)) {
        statuses[id] = fakeStatus(id, "idle");
      }
      return { statuses };
    });
  },
}));

async function notifyTransition(
  config: AppConfig | null,
  prev: TunnelState | undefined,
  status: TunnelStatus,
): Promise<void> {
  if (!config || prev === status.state) return;
  const prefs = config.notifications;
  const name = config.tunnels.find((t) => t.id === status.id)?.name ?? "Tunnel";
  if (status.state === "connected" && prefs.onConnect) {
    await notify(name, "Connected");
  } else if (status.state === "error" && prefs.onError) {
    await notify(name, status.message ?? "Error");
  } else if (
    status.state === "idle" &&
    prefs.onDisconnect &&
    (prev === "connected" || prev === "reconnecting")
  ) {
    await notify(name, "Disconnected");
  }
}

function fakeStatus(id: string, state: TunnelStatus["state"]): TunnelStatus {
  return {
    id,
    state,
    message: null,
    stats: {
      bytesUp: 0,
      bytesDown: 0,
      activeConnections: 0,
      uptimeSeconds: 0,
      retryCount: 0,
    },
    since: new Date().toISOString(),
  };
}
