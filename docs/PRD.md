# Product Requirements — Tunnelium

## 1. One-line pitch

A friendly, open-source, cross-platform desktop app that makes SSH port forwarding —
local, remote, and dynamic — easy to set up, organize, and keep running.

## 2. Why build it

MobaXterm's tunnel manager is loved but Windows-only, closed-source, and bundled
inside a much larger tool. Command-line `ssh -L/-R/-D` is powerful but forgettable and
tedious to manage for many tunnels. There is room for a **focused, cross-platform,
open-source tunnel manager** that:

- runs on Linux, Windows, and macOS from one codebase,
- explains what each option does in plain language,
- keeps many tunnels organized and auto-reconnecting in the background,
- stores secrets safely and verifies host keys by default.

## 3. Target users (personas)

| Persona | Needs |
| --- | --- |
| **Dev "Sam"** | Reach a remote Postgres/Redis bound to `localhost`; keep 3–4 tunnels alive all day; auto-reconnect after Wi-Fi drops. |
| **Sysadmin "Riya"** | Manage dozens of tunnels across bastions/jump hosts; group by environment; start-all at shift start. |
| **Privacy user "Jordan"** | One-click SOCKS proxy to browse safely on public Wi-Fi; route only the browser through it. |
| **Newcomer "Alex"** | Doesn't know L vs R vs D; needs the UI + guide to teach the concept with real scenarios. |

## 4. Goals & non-goals

### Goals
- Best-in-class **tunnel management UX** across all three OSes.
- **All three forwarding modes** working reliably, including many concurrent tunnels.
- **Robust background operation**: auto-start, auto-reconnect, tray, notifications.
- **Security-first defaults**: host-key verification on, secrets in OS keychain.
- **Teach the user**: inline help + a standalone plain-English guide.

### Non-goals (at least for v1)
- Full terminal / SSH shell emulator (this is a *tunnel* manager, not a client like PuTTY).
- SFTP file browser.
- Mobile apps.
- Team/cloud sync of configs (local-first only in v1; export/import covers sharing).

## 5. Feature list

Legend: **[MVP]** = first runnable milestone, **[v1]** = target for 1.0, **[later]** = post-1.0.

### 5.1 Forwarding modes
- **[MVP]** Local port forwarding (`-L`): listen locally, forward to a target reachable from the SSH server.
- **[v1]** Remote port forwarding (`-R`): listen on the SSH server, forward back to a target reachable from this machine.
- **[v1]** Dynamic port forwarding (`-D`): local **SOCKS5** proxy over SSH.

### 5.2 Tunnel definition / editing
- **[MVP]** Create, edit, delete a tunnel.
- **[v1]** Duplicate a tunnel; enable/disable without deleting.
- **[MVP]** Name + optional description.
- **[v1]** Groups/folders + tags; search & filter the list.

### 5.3 Connection & auth
- **[MVP]** SSH host, port, username.
- **[MVP]** Auth: **private key** (path or imported) with **passphrase**.
- **[v1]** Auth: **password**, **SSH agent**, **keyboard-interactive**.
- **[v1]** **Host-key verification** against `known_hosts` (trust-on-first-use prompt).
- **[v1]** Keepalive interval, connection timeout, compression toggle.

### 5.4 Networking options
- **[MVP]** Listen/bind address — bind to `127.0.0.1`, `0.0.0.0`, or a **specific local network adapter/IP**.
- **[MVP]** Listen port; target (forward) host + port.
- **[v1]** **Proxy** to reach the SSH server: HTTP CONNECT or SOCKS5 (with optional auth).
- **[v1]** **Jump hosts / bastions** (ProxyJump), chained.

### 5.5 Lifecycle & automation
- **[MVP]** Start / stop a single tunnel; live status (idle/connecting/connected/error).
- **[v1]** **Start all** / **stop all**.
- **[v1]** **Auto-start** selected tunnels when the app launches.
- **[v1]** **Auto-reconnect** with exponential backoff + max-retry cap; manual retry.
- **[v1]** **Start app on system boot** (OS autostart).

### 5.6 Observability
- **[MVP]** Per-tunnel event log (connect, disconnect, error, retries).
- **[v1]** Live log viewer panel; copy/export logs.
- **[v1]** Traffic stats: bytes up/down, uptime, active connection count.
- **[v1]** Desktop **notifications** on connect/disconnect/failure (toggleable).

### 5.7 App-level
- **[v1]** System **tray** with quick start/stop + minimize-to-tray.
- **[v1]** **Dark / light / system** theme.
- **[v1]** Global settings & per-tunnel defaults.
- **[v1]** **Import / export** config as JSON (**secrets excluded**; re-entered on import).
- **[later]** Config backup/restore; CLI companion; per-tunnel bandwidth graphs.

### 5.8 Security (cross-cutting, all [v1])
- Secrets (passwords, passphrases) stored in **OS keychain**, never plaintext on disk.
- Host-key verification enabled by default.
- No secrets in exported config, logs, or crash reports.
- Input validation on every field (ports, hosts, paths) before use.

## 6. Success criteria for v1

- All three forwarding modes work with ≥ 10 concurrent tunnels stable for hours.
- Auto-reconnect recovers within backoff window after a network drop, no leaks.
- Cold-start to running tunnel in **< 3 clicks** for a returning user.
- Installers build in CI for all three OSes.
- Newcomer can set up their first local tunnel using only the in-app help + guide.
- Test coverage ≥ 80% on the Rust tunnel engine and config/model layers.

## 7. Decisions

1. **App name** — Tunnelium.
2. **Frontend UI kit** — shadcn/ui + Tailwind.
3. **Companion CLI** — deferred, not in v1.
4. **License** — MIT.
