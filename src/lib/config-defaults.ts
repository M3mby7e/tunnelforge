import type { AppConfig } from "@/types/generated/AppConfig";
import type { ReconnectPolicy } from "@/types/generated/ReconnectPolicy";

/** Mirrors the Rust `ReconnectPolicy::default()`. */
export function reconnectDefault(): ReconnectPolicy {
  return {
    enabled: true,
    initialDelayMs: 1000,
    maxDelayMs: 60000,
    factor: 2,
    jitter: true,
    maxRetries: null,
  };
}

/** Mirrors the Rust `AppConfig::default()`. */
export function defaultAppConfig(): AppConfig {
  return {
    version: 1,
    theme: "system",
    startOnBoot: false,
    minimizeToTray: true,
    notifications: { onConnect: false, onDisconnect: false, onError: true },
    defaults: {},
    tunnels: [],
  };
}
