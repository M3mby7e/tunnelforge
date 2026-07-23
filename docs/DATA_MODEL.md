# Data Model — Tunnelium

The Rust `model/*` types are the source of truth. Below is the conceptual schema in
TypeScript-ish notation for readability; the JSON on disk matches it (secrets excluded).

## 1. Core types

```ts
type ForwardKind = "local" | "remote" | "dynamic";

interface ListenSpec {
  // Which local network adapter/address to bind the listener to.
  bindAddress: string;   // "127.0.0.1" (default) | "0.0.0.0" | a specific NIC IP
  port: number;          // 1..=65535 (0 = auto-pick for local/dynamic)
}

interface ForwardTarget {
  // Where traffic ultimately goes. Not used for "dynamic" (client picks per-connection).
  host: string;          // e.g. "127.0.0.1", "db.internal"
  port: number;          // 1..=65535
}

interface SshEndpoint {
  host: string;
  port: number;          // default 22
  username: string;
}

type AuthMethod =
  | { kind: "password"; secretRef: string }                 // secretRef → keychain id
  | { kind: "privateKey"; keyPath: string; passphraseRef?: string }
  | { kind: "privateKeyInline"; keyRef: string; passphraseRef?: string } // imported key body in keychain
  | { kind: "agent" }                                       // use SSH agent
  | { kind: "keyboardInteractive"; secretRef?: string };

interface ProxyConfig {
  kind: "http" | "socks5";
  host: string;
  port: number;
  authRef?: string;      // optional proxy creds → keychain id ("user\npass")
}

interface JumpHost {
  endpoint: SshEndpoint;
  auth: AuthMethod;
}

interface ReconnectPolicy {
  enabled: boolean;
  initialDelayMs: number;   // e.g. 1000
  maxDelayMs: number;       // e.g. 60000
  factor: number;           // e.g. 2.0
  jitter: boolean;          // true
  maxRetries: number | null;// null = unlimited
}

interface TunnelConfig {
  id: string;               // uuid v4
  name: string;
  description?: string;
  kind: ForwardKind;

  enabled: boolean;         // disabled tunnels are ignored by start-all / autostart
  autoStart: boolean;       // start when the app launches
  reconnect: ReconnectPolicy;

  ssh: SshEndpoint;
  auth: AuthMethod;

  listen: ListenSpec;       // for local/dynamic: the local listener
                            // for remote: the address on the SSH server to bind
  target?: ForwardTarget;   // local: destination reachable from server
                            // remote: destination reachable from this machine
                            // dynamic: omitted

  proxy?: ProxyConfig;      // reach the SSH server through this proxy
  jumpHosts?: JumpHost[];   // ProxyJump chain, in order

  keepAliveSeconds?: number;// server keepalive interval (0/undefined = off)
  connectTimeoutMs?: number;
  compression?: boolean;

  group?: string;           // folder name
  tags?: string[];

  createdAt: string;        // ISO-8601
  updatedAt: string;
}

interface AppConfig {
  version: 1;
  theme: "system" | "light" | "dark";
  startOnBoot: boolean;
  minimizeToTray: boolean;
  notifications: { onConnect: boolean; onDisconnect: boolean; onError: boolean };
  defaults: Partial<Pick<TunnelConfig, "ssh" | "reconnect" | "keepAliveSeconds">>;
  tunnels: TunnelConfig[];
}
```

## 2. Runtime status (not persisted)

```ts
type TunnelState =
  | "idle" | "connecting" | "connected" | "reconnecting" | "error" | "stopping";

interface StatsSnapshot {
  bytesUp: number;
  bytesDown: number;
  activeConnections: number;
  uptimeSeconds: number;
  retryCount: number;
}

interface TunnelStatus {
  id: string;
  state: TunnelState;
  message?: string;         // last error / info, UI-friendly
  stats: StatsSnapshot;
  since: string;            // ISO-8601 of last state change
}
```

Backend → UI events (over Tauri event bus):
`tunnel://status` (TunnelStatus), `tunnel://log` ({id, level, ts, line}),
`tunnel://stats` (throttled StatsSnapshot).

## 3. Persistence

- **Config file:** `config.json` in the OS app-config dir (via Tauri path API), e.g.
  `~/.config/Tunnelium/` (Linux), `~/Library/Application Support/Tunnelium/` (macOS),
  `%APPDATA%\Tunnelium\` (Windows).
- **Atomic writes:** write to `config.json.tmp`, then rename — never a partial file.
- **`known_hosts`:** app-local known_hosts file in the same dir.
- **Secrets:** OS keychain via `keyring`, service = `Tunnelium`, account = the `*Ref`
  id (e.g. `tunnel:<uuid>:password`). The config file holds only the ref id.

## 4. Import / export

- **Export:** the full `AppConfig` as JSON **with every `*Ref` field blanked and secrets
  omitted**. Safe to commit/share.
- **Import:** merges tunnels (new ids), then prompts the user to re-enter any secrets the
  imported tunnels need. Validation runs on every field before the import is accepted.

## 5. Validation rules (enforced at the IPC boundary)

- `port`, `ssh.port`, `listen.port`, `target.port`, `proxy.port` ∈ `1..=65535`
  (`listen.port` may be `0` for local/dynamic to auto-pick).
- `bindAddress` is a valid IP literal or one of the known local interface addresses.
- `host` fields are non-empty and are valid hostnames or IP literals.
- `keyPath` exists and is readable; no path traversal is smuggled through it.
- `name` non-empty and unique-per-group (soft warning if duplicated).
- `reconnect.factor >= 1.0`, delays non-negative, `initialDelayMs <= maxDelayMs`.
