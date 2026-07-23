import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppConfig } from "@/types/generated/AppConfig";
import type { LogLine } from "@/types/generated/LogLine";
import type { NetworkInterface } from "@/types/generated/NetworkInterface";
import type { TunnelStatus } from "@/types/generated/TunnelStatus";

/** True when running inside the Tauri native shell (false in a plain browser). */
export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  await invoke("save_config", { config });
}

export async function setSecret(account: string, secret: string): Promise<void> {
  await invoke("set_secret", { account, secret });
}

export async function clearSecret(account: string): Promise<void> {
  await invoke("clear_secret", { account });
}

export async function forgetHostKey(host: string, port: number): Promise<void> {
  await invoke("forget_host_key", { host, port });
}

export async function listNetworkInterfaces(): Promise<NetworkInterface[]> {
  return invoke<NetworkInterface[]>("list_network_interfaces");
}

export async function notify(title: string, body: string): Promise<void> {
  if (!isTauri) return;
  const { isPermissionGranted, requestPermission, sendNotification } =
    await import("@tauri-apps/plugin-notification");
  let granted = await isPermissionGranted();
  if (!granted) granted = (await requestPermission()) === "granted";
  if (granted) sendNotification({ title, body });
}

export async function setStartOnBoot(on: boolean): Promise<void> {
  if (!isTauri) return;
  const { enable, disable } = await import("@tauri-apps/plugin-autostart");
  if (on) await enable();
  else await disable();
}

export async function getStartOnBoot(): Promise<boolean> {
  if (!isTauri) return false;
  const { isEnabled } = await import("@tauri-apps/plugin-autostart");
  return isEnabled();
}

export async function startTunnel(id: string): Promise<void> {
  await invoke("start_tunnel", { id });
}

export async function stopTunnel(id: string): Promise<void> {
  await invoke("stop_tunnel", { id });
}

export async function startAllTunnels(): Promise<void> {
  await invoke("start_all_tunnels");
}

export async function stopAllTunnels(): Promise<void> {
  await invoke("stop_all_tunnels");
}

export async function onStatus(
  handler: (status: TunnelStatus) => void,
): Promise<UnlistenFn> {
  return listen<TunnelStatus>("tunnel://status", (event) =>
    handler(event.payload),
  );
}

export async function onLog(
  handler: (line: LogLine) => void,
): Promise<UnlistenFn> {
  return listen<LogLine>("tunnel://log", (event) => handler(event.payload));
}
