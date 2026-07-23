# Tunnelium

A free, open-source, cross-platform **SSH tunnel manager** for Linux, Windows, and
macOS. Create, organize, and run local / remote / dynamic SSH port forwards from a
clean desktop UI — think of it as a friendly, open alternative to MobaXterm's tunnel
manager, focused purely on tunneling done well.

Built with **Tauri 2 + Rust** (small, fast, secure native binaries) and a **React +
TypeScript** UI.

---

## Status

✅ **Feature-complete.** All three forwarding modes (local / remote / dynamic-SOCKS5),
full tunnel management, live traffic stats, auto-reconnect, auto-start, every SSH auth
method (key / password / agent / keyboard-interactive), host-key verification, HTTP/SOCKS
proxy, jump hosts, a bind-adapter picker, system tray, desktop notifications, a settings
screen, and a per-tunnel log viewer are implemented and tested (47 unit + 4 end-to-end
tests). Cross-platform installers build in CI. See the [Roadmap](docs/ROADMAP.md).

| Document | What it covers |
| --- | --- |
| [Product Requirements](docs/PRD.md) | Goals, personas, the full feature list, scope |
| [Architecture](docs/ARCHITECTURE.md) | Tech stack, module layout, the tunnel engine, security model |
| [Data Model](docs/DATA_MODEL.md) | Config schema, persistence, how secrets are stored |
| [Roadmap](docs/ROADMAP.md) | Phased build plan, milestones, task breakdown |
| [**Tunneling Guide**](docs/TUNNELING_GUIDE.md) | **Plain-English explanation of every option, with real-life scenarios** |

New to SSH tunnels? **Start with the [Tunneling Guide](docs/TUNNELING_GUIDE.md)** — it
explains what each type of forwarding is for using everyday examples, before you touch
a single setting.

---

## Features

- **Local**, **remote**, and **dynamic (SOCKS5)** port forwarding
- Add / edit / duplicate / delete tunnels, organized into groups with search
- **Start all** / **stop all**, plus per-tunnel start/stop with live status
- **Auto-start** (on app launch) and **auto-reconnect** (with backoff) per tunnel
- Multiple auth methods: **password, private key (+ passphrase), SSH agent, keyboard-interactive**
- Bind to a specific **local network adapter** / address
- Connect through an **HTTP or SOCKS proxy**, and via **jump hosts (bastions)**
- **Host-key verification** (known_hosts) — secrets kept in the **OS keychain**, never in plaintext
- **System tray**, start-on-boot, desktop notifications, dark/light themes
- Per-tunnel **live logs** and traffic stats
- **Import / export** configuration (secrets excluded)

The complete, categorized list lives in the [PRD](docs/PRD.md#feature-list).

---

## Installation

### Download a release (easiest)

Grab the installer for your OS from the
[**Releases**](https://github.com/M3mby7e/tunnelium/releases) page:

| OS | File |
| --- | --- |
| **macOS** | `Tunnelium_x.y.z_aarch64.dmg` (Apple Silicon) or `..._x64.dmg` (Intel) |
| **Windows** | `Tunnelium_x.y.z_x64-setup.exe` or `..._x64_en-US.msi` |
| **Linux** | `Tunnelium_x.y.z_amd64.AppImage` (portable) or `..._amd64.deb` / `...rpm` |

> **Builds are currently unsigned.** First launch is blocked by the OS gatekeeper:
> - **macOS:** right-click the app → **Open**, then confirm (or `xattr -dr com.apple.quarantine /Applications/Tunnelium.app`).
> - **Windows:** SmartScreen → **More info** → **Run anyway**.
> - **Linux (AppImage):** `chmod +x Tunnelium_*.AppImage` then run it.

### Build from source

Prerequisites: **Node.js ≥ 20**, **Rust (stable)** via [rustup](https://rustup.rs),
and the [Tauri OS prerequisites](https://tauri.app/start/prerequisites/).

```bash
git clone https://github.com/M3mby7e/tunnelium.git
cd tunnelium
npm install
npm run tauri dev      # run the app
npm run tauri build    # produce an installer in src-tauri/target/release/bundle/
```

Releases are built automatically by the [release workflow](.github/workflows/release.yml)
on every `v*` tag, across Linux, Windows, and macOS.

---

## License

**MIT** — see [`LICENSE`](LICENSE).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). The short version: small focused PRs, tests
for new behavior, and no secrets in commits.
