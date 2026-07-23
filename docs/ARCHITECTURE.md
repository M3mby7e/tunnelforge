# Architecture — Tunnelium

## 1. Tech stack

| Layer | Choice | Why |
| --- | --- | --- |
| App shell | **Tauri 2** | Tiny native binaries, real OS installers, secure IPC, tray/notifications/autostart plugins. |
| Core / backend | **Rust** (async, Tokio) | Memory-safe systems language; ideal for a long-running networking daemon. |
| SSH | **`russh`** + **`russh-keys`** | Pure-Rust async SSH client; supports direct-tcpip, tcpip-forward, agent, keyboard-interactive — everything the three forward modes need. |
| Secrets | **`keyring`** crate | OS keychain: macOS Keychain, Windows Credential Manager, Linux Secret Service. |
| UI | **React + TypeScript + Vite** | Large ecosystem, matches team review tooling; typed IPC. |
| UI styling/state | **Tailwind CSS**, **zustand** (state), **react-hook-form + zod** (forms/validation) | Lightweight, well-understood. Final UI kit is an open question in the PRD. |
| Tauri plugins | `tauri-plugin-autostart`, `tauri-plugin-notification`, `tauri-plugin-store` (settings), `tauri-plugin-single-instance` | Official, cross-platform. |

> **russh vs shelling out to system `ssh`:** we use the `russh` library, not
> `Command::new("ssh")`. System `ssh` is not guaranteed on Windows, gives us no
> structured status, and makes reconnect/stats hard. An in-process SSH client gives
> full control over lifecycle, per-tunnel state, bind address, and metrics. API details
> get verified against current `russh` docs (via Context7) during Phase 2.

## 2. High-level shape

```
┌───────────────────────────── Tauri app ─────────────────────────────┐
│                                                                      │
│  React/TS UI  ──invoke()──►  Tauri commands  ──►  Tunnel engine      │
│      ▲                          (IPC layer)          (Rust/Tokio)    │
│      └────────── events ◄──── event bus  ◄──── status/log emitters   │
│                                                                      │
│  Persistence: JSON config file  +  OS keychain  +  known_hosts       │
└──────────────────────────────────────────────────────────────────────┘
```

- The **UI never touches SSH**. It only calls typed commands and listens for events.
- The **tunnel engine** owns all live state and runs each tunnel as an async task.
- **Secrets** flow UI → keychain by *reference*; the config file stores only a keychain
  key id, never the secret itself.

## 3. Backend module layout (`src-tauri/src/`)

Small, focused files (per coding-style: 200–400 lines typical). Immutable config
structs; state changes produce new values rather than mutating in place.

```
main.rs                     App entry: Tauri builder, tray, plugins, single-instance.
lib.rs                      Wires modules; exposes the command handler set.

commands/                   IPC boundary — thin, validate input, delegate to engine.
  mod.rs
  tunnel_cmd.rs             create/update/delete/duplicate, start/stop, start_all/stop_all
  config_cmd.rs             load/save, import/export
  secret_cmd.rs             set/clear secret (keychain), never returns secrets
  system_cmd.rs             list network interfaces, toggle OS autostart

tunnel/                     Core engine.
  mod.rs
  manager.rs                TunnelManager: registry of tunnels; start_all/stop_all; emits events
  runtime.rs                Per-tunnel async task: state machine (Idle→Connecting→Connected→Error)
  session.rs                russh session setup: transport, auth, keepalive, host-key check
  auth.rs                   password / private-key / agent / keyboard-interactive
  hostkey.rs                known_hosts load + verify + TOFU decision
  reconnect.rs              backoff policy (exponential + jitter, max retries)
  proxy.rs                  dial SSH host via HTTP CONNECT or SOCKS5 (returns a stream)
  jump.rs                   ProxyJump chaining (session over direct-tcpip of previous hop)
  forward/
    mod.rs
    local.rs                -L: local listener → direct-tcpip channel
    remote.rs               -R: tcpip-forward request → handle forwarded-tcpip channels
    dynamic.rs              -D: local SOCKS5 server → per-conn direct-tcpip channel
  stats.rs                  byte counters, uptime, active-conn gauge (atomic, lock-light)

model/                      Plain data types (serde). No behavior, no I/O.
  mod.rs
  tunnel_config.rs          TunnelConfig, ForwardKind, ListenSpec, ForwardTarget
  auth_config.rs            AuthConfig, AuthMethod, secret references
  proxy_config.rs           ProxyConfig, JumpHost
  app_config.rs             AppConfig (theme, defaults, autostart flags)
  status.rs                 TunnelStatus, TunnelEvent, StatsSnapshot

store/                      Persistence.
  mod.rs
  config_store.rs           atomic read/write of config JSON (temp-file + rename)
  keychain_store.rs         keyring wrapper: set/get/delete by key id
  paths.rs                  resolve config/known_hosts/log dirs via Tauri path API

telemetry/
  log.rs                    structured per-tunnel log ring buffer + tracing
error.rs                    typed error enum (thiserror); maps to UI-friendly messages
```

## 4. Frontend module layout (`src/`)

```
main.tsx / App.tsx
api/
  client.ts                 typed invoke() wrapper + error normalization
  tunnels.ts | config.ts | system.ts | secrets.ts
  events.ts                 subscribe to backend events (status, log, stats)
state/
  tunnelStore.ts            zustand store mirroring backend state
components/
  layout/                   Sidebar (groups), Toolbar (start-all/stop-all), StatusBar
  tunnels/                  TunnelList, TunnelRow, StatusBadge, QuickActions
  form/                     TunnelForm (all options, sectioned), field help tooltips
  logs/                     LogViewer, StatsPanel
  settings/                 Settings, Theme, Defaults
  common/                   Modal, Toast, Tooltip, ConfirmDialog
hooks/
  useTunnels.ts | useTunnelEvents.ts | useNetworkInterfaces.ts
help/
  copy.ts                   the plain-English help strings shown inline (source of truth
                            shared with docs/TUNNELING_GUIDE.md)
types/                      TS types mirroring Rust model (kept in sync; see §7)
```

## 5. Tunnel engine design

### 5.1 Per-tunnel state machine (`runtime.rs`)

```
Idle ──start──► Connecting ──ok──► Connected
  ▲                │  fail                │ drop / error
  │                ▼                      ▼
  └──stop──── Reconnecting ◄────────── (if auto-reconnect)
                   │ give up (max retries)
                   ▼
                 Error
```

Each tunnel runs as one supervised Tokio task. Stop = cancel the task + close listeners
and channels. The manager holds a `JoinHandle` + a cancellation token per tunnel.

### 5.2 The three forward modes over `russh`

- **Local (`-L`)** — bind a `TcpListener` on the chosen adapter/port; for each inbound
  connection open a `direct-tcpip` channel to `(forwardHost, forwardPort)` and pump bytes
  both ways.
- **Remote (`-R`)** — send a `tcpip-forward` global request to the server; in the client
  handler, accept incoming `forwarded-tcpip` channels and connect each to the local target.
- **Dynamic (`-D`)** — run a minimal **SOCKS5** server on the local listener; for each
  SOCKS CONNECT, open a `direct-tcpip` channel to the SOCKS-requested destination.

All three share the same session/auth/keepalive/host-key code in `session.rs`.

### 5.3 Reconnect

`reconnect.rs` implements exponential backoff with jitter and a configurable cap. On an
unexpected drop and `autoReconnect = true`, the runtime transitions to `Reconnecting`,
re-establishes the session, and re-arms listeners. Byte counters persist across
reconnects; uptime resets.

## 6. Security model

- **Secrets never on disk in plaintext.** Config JSON stores a keychain **key id**; the
  actual password/passphrase lives in the OS keychain (`keyring`). Exported config omits
  secrets entirely.
- **Host-key verification on by default.** First connection to an unknown host prompts a
  trust decision (fingerprint shown); the accepted key is pinned in `known_hosts`. A
  changed key blocks the connection with a warning.
- **Input validation at the IPC boundary** (`commands/*`): ports in range, hostnames/IPs
  well-formed, key file readable, no path traversal on file inputs.
- **Least surprise on binding:** binding to `0.0.0.0` (LAN-exposed) shows a warning in the
  UI, since it makes the forwarded service reachable by others on the network.
- **No secrets in logs.** The log layer redacts credentials and never records passphrases.

## 7. Keeping Rust ↔ TypeScript types in sync

Rust `model/*` is the source of truth. We generate/mirror TS types (candidate:
`ts-rs` or `tauri-specta`) so the UI and backend can't drift. Decision recorded in the
Roadmap's Phase 1.

## 8. Testing strategy (see testing.md)

- **Unit** — model validation, backoff math, SOCKS5 parsing, host-key decisions, config
  (de)serialization. Pure and fast.
- **Integration** — spin an in-process SSH server (`russh` server side) as a fixture; drive
  a real local/remote/dynamic forward through a loopback target; assert bytes round-trip and
  reconnect works.
- **E2E** — Tauri UI smoke tests for the critical flow: create → start → status → stop.
- Target ≥ 80% on engine + model layers.
