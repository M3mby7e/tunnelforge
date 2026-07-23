import type { AppConfig } from "@/types/generated/AppConfig";
import type { TunnelConfig } from "@/types/generated/TunnelConfig";
import type { TunnelState } from "@/types/generated/TunnelState";
import type { TunnelStatus } from "@/types/generated/TunnelStatus";
import { reconnectDefault } from "@/lib/config-defaults";

/**
 * Demo data used ONLY when running outside the Tauri shell (a plain browser),
 * so the UI has something to render during design work. The real app loads
 * actual config from the backend.
 */

function tunnel(
  partial: Partial<TunnelConfig> & Pick<TunnelConfig, "id" | "name">,
): TunnelConfig {
  const now = new Date().toISOString();
  return {
    description: null,
    kind: "local",
    enabled: true,
    autoStart: false,
    reconnect: reconnectDefault(),
    ssh: { host: "example.com", port: 22, username: "sam" },
    auth: { kind: "agent" },
    listen: { bindAddress: "127.0.0.1", port: 5432 },
    target: { host: "db.internal", port: 5432 },
    proxy: null,
    jumpHosts: [],
    keepAliveSeconds: null,
    connectTimeoutMs: null,
    compression: null,
    group: "Production",
    tags: [],
    createdAt: now,
    updatedAt: now,
    ...partial,
  };
}

const DEMO_TUNNELS: TunnelConfig[] = [
  tunnel({
    id: "11111111-1111-1111-1111-111111111111",
    name: "Prod Postgres",
    ssh: { host: "bastion.acme.io", port: 22, username: "sam" },
    listen: { bindAddress: "127.0.0.1", port: 5432 },
    target: { host: "db.internal", port: 5432 },
    group: "Production",
  }),
  tunnel({
    id: "22222222-2222-2222-2222-222222222222",
    name: "Office intranet",
    ssh: { host: "gateway.acme.io", port: 22, username: "sam" },
    listen: { bindAddress: "127.0.0.1", port: 8080 },
    target: { host: "10.0.0.5", port: 8080 },
    group: "Production",
  }),
  tunnel({
    id: "33333333-3333-3333-3333-333333333333",
    name: "Staging Redis",
    ssh: { host: "jump.staging.io", port: 22, username: "riya" },
    listen: { bindAddress: "127.0.0.1", port: 6379 },
    target: { host: "redis.staging", port: 6379 },
    group: "Staging",
  }),
  tunnel({
    id: "44444444-4444-4444-4444-444444444444",
    name: "SOCKS proxy",
    kind: "dynamic",
    ssh: { host: "vps.example.net", port: 22, username: "jordan" },
    listen: { bindAddress: "127.0.0.1", port: 1080 },
    target: null,
    group: "Personal",
  }),
  tunnel({
    id: "55555555-5555-5555-5555-555555555555",
    name: "Demo webhook",
    kind: "remote",
    ssh: { host: "vps.example.net", port: 22, username: "jordan" },
    listen: { bindAddress: "0.0.0.0", port: 9000 },
    target: { host: "localhost", port: 3000 },
    group: "Personal",
  }),
];

export function demoConfig(): AppConfig {
  return {
    version: 1,
    theme: "system",
    startOnBoot: false,
    minimizeToTray: true,
    notifications: { onConnect: false, onDisconnect: false, onError: true },
    defaults: {},
    tunnels: DEMO_TUNNELS,
  };
}

function status(
  id: string,
  state: TunnelState,
  stats?: Partial<TunnelStatus["stats"]>,
  message?: string,
): TunnelStatus {
  return {
    id,
    state,
    message: message ?? null,
    stats: {
      bytesUp: 0,
      bytesDown: 0,
      activeConnections: 0,
      uptimeSeconds: 0,
      retryCount: 0,
      ...stats,
    },
    since: new Date().toISOString(),
  };
}

export function demoStatuses(): Record<string, TunnelStatus> {
  return {
    "11111111-1111-1111-1111-111111111111": status(
      "11111111-1111-1111-1111-111111111111",
      "connected",
      {
        bytesUp: 1_258_291,
        bytesDown: 19_293_798,
        activeConnections: 2,
        uptimeSeconds: 11_532,
      },
    ),
    "33333333-3333-3333-3333-333333333333": status(
      "33333333-3333-3333-3333-333333333333",
      "error",
      undefined,
      "Connection refused (port 22)",
    ),
  };
}
