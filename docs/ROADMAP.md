# Roadmap — Tunnelium

Phased build plan. Each phase ends with tests + a code-review pass (per the review and
testing rules) and a working, demonstrable increment.

## Progress

✅ **Implemented:** all three forwarding modes (local / remote / dynamic-SOCKS5), tunnel
CRUD + duplicate + enable/disable, live traffic stats, auto-reconnect with backoff,
auto-start on launch, all auth methods (key / password / agent / keyboard-interactive),
host-key TOFU + forget-host-key, HTTP/SOCKS proxy, jump hosts (ProxyJump), bind-adapter
picker, system tray, desktop notifications, settings screen, start-on-boot, per-tunnel
log viewer, config import/export, and cross-platform installers via CI.

Tested: 47 Rust unit tests + 4 end-to-end tests (real in-process SSH server proving bytes
round-trip through local, remote, dynamic, and jump-host tunnels).

⬜ **Future:** imported-key (inline) auth, live-OTP keyboard-interactive prompts, bandwidth
graphs, a companion CLI.

---

## Phase 0 — Repo & scaffold
**Outcome:** empty-but-real app that launches a window on all three OSes.

- [ ] `git init`, `.gitignore`, MIT `LICENSE`, `CONTRIBUTING.md`, issue/PR templates.
- [ ] Scaffold Tauri 2 + React + TS + Vite; Tailwind; zustand; ESLint/Prettier; rustfmt/clippy.
- [ ] CI (GitHub Actions): lint + test on Linux/Windows/macOS.
- [ ] Decide Rust↔TS type-gen tool (`ts-rs` vs `tauri-specta`) and wire it.

## Phase 1 — Model, config store, secrets
**Outcome:** config persists; secrets go to the OS keychain; types shared with UI.

- [ ] `model/*` types + serde + validation (unit-tested).
- [ ] `store/config_store.rs` atomic read/write; `store/paths.rs`.
- [ ] `store/keychain_store.rs` via `keyring` (set/get/delete).
- [ ] Commands: `load_config`, `save_config`, `set_secret`, `clear_secret`.

## Phase 2 — SSH session + local forwarding (**MVP**)
**Outcome:** create one local tunnel, start it, move real bytes, stop it.

- [ ] `tunnel/session.rs`: russh connect + private-key auth + host-key check (TOFU).
- [ ] `tunnel/forward/local.rs`: listener → direct-tcpip pump.
- [ ] `tunnel/runtime.rs` state machine; `tunnel/manager.rs` (single tunnel).
- [ ] Commands + events: `start_tunnel`, `stop_tunnel`, `tunnel://status`, `tunnel://log`.
- [ ] Minimal UI: TunnelForm (local only) + TunnelList + StatusBadge.
- [ ] Integration test: in-process russh server, loopback target, bytes round-trip.

## Phase 3 — Remote + dynamic forwarding
- [ ] `forward/remote.rs` (tcpip-forward + forwarded-tcpip handling).
- [ ] `forward/dynamic.rs` (SOCKS5 server → direct-tcpip).
- [ ] UI: kind selector + mode-specific fields; help tooltips.
- [ ] Integration tests for remote and dynamic.

## Phase 4 — Full tunnel management
- [ ] CRUD: create/edit/delete/duplicate; enable/disable.
- [ ] **Start all / stop all**; groups, tags, search/filter.
- [ ] `stats.rs`: bytes up/down, uptime, active conns; `tunnel://stats` events; StatsPanel.

## Phase 5 — Auth & host-key hardening
- [ ] `auth.rs`: password, SSH agent, keyboard-interactive (key already done).
- [ ] `hostkey.rs`: known_hosts pin, changed-key block, fingerprint prompt UI.
- [ ] Passphrase/password entry → keychain flow end-to-end.

## Phase 6 — Reconnect, autostart, proxy, jump hosts, bind adapter
- [ ] `reconnect.rs` backoff + jitter + cap; auto-reconnect wired to runtime.
- [ ] Auto-start on app launch; `tauri-plugin-autostart` for start-on-boot.
- [ ] `proxy.rs` (HTTP CONNECT / SOCKS5 to reach SSH host).
- [ ] `jump.rs` ProxyJump chaining.
- [ ] Network-interface enumeration → bind-address picker UI.

## Phase 7 — App polish
- [ ] System tray (quick start/stop, minimize-to-tray), single-instance.
- [ ] Desktop notifications (connect/disconnect/error, toggleable).
- [ ] Dark/light/system theme; settings screen; per-tunnel defaults.
- [ ] Live log viewer (copy/export); import/export config (secrets excluded).

## Phase 8 — Packaging & release
- [ ] Tauri bundles: `.AppImage` + `.deb`/`.rpm`, `.msi`/`.exe`, `.dmg`.
- [ ] Signing/notarization notes (macOS), Windows signing guidance.
- [ ] GitHub Actions release workflow; auto-attach installers to tagged releases.
- [ ] Fill in README install section; publish first release.

## Phase 9 — Quality gate & open-source launch
- [ ] Coverage ≥ 80% on engine + model; clippy/eslint clean.
- [ ] Security review pass (secrets, host-key, bind warnings, input validation).
- [ ] Finalize docs, screenshots/GIFs, and the Tunneling Guide.
- [ ] Tag `v1.0.0`, publish to GitHub.

---

## Milestone summary

| Milestone | Phases | Delivers |
| --- | --- | --- |
| **M1 – MVP** | 0–2 | One local tunnel, start/stop, persisted config, keychain secret. |
| **M2 – All modes** | 3–4 | Local/remote/dynamic + full list management + start-all/stop-all + stats. |
| **M3 – Production** | 5–7 | All auth, host-key security, reconnect, autostart, proxy, jump, tray, polish. |
| **M4 – Release** | 8–9 | Installers, CI releases, 80% coverage, v1.0 on GitHub. |

## Cross-cutting definition of done (every phase)
- Tests written first where practical (TDD), ≥ 80% on new engine/model code.
- `cargo clippy` + `cargo fmt` + ESLint/Prettier clean.
- Code-review pass; no CRITICAL/HIGH open.
- No secrets in code, logs, or exported config.
