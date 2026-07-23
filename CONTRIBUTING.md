# Contributing to Tunnelium

Thanks for your interest! Tunnelium is an open-source, cross-platform SSH tunnel
manager built with **Tauri 2 + Rust** (core) and **React + TypeScript** (UI).

## Ground rules

- **Small, focused PRs.** One concern per pull request.
- **Tests for new behavior.** The Rust tunnel engine and model/config layers target ≥ 80%
  coverage. Prefer writing the test first (TDD).
- **No secrets in commits.** No passwords, keys, tokens, or private hosts — ever. See
  [SECURITY](#security).
- **Keep it readable.** Small files, clear names, early returns, no deep nesting.
- **Immutability.** Prefer building new values over mutating in place.

## Prerequisites

- **Node.js** ≥ 20 and **npm**
- **Rust** (stable) via [rustup](https://rustup.rs)
- Tauri OS prerequisites: <https://tauri.app/start/prerequisites/>

## Getting started

```bash
git clone <your-fork-url>
cd tunnelium
npm install
npm run tauri dev        # launches the desktop app in dev mode
```

Other useful commands:

```bash
npm run lint             # ESLint (frontend)
npm run format           # Prettier
npm run build            # type-check + build frontend
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml
```

## Commit messages

Conventional-commits style:

```
<type>: <description>

<optional body>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`.

## Pull request checklist

- [ ] Lint + format clean (`npm run lint`, `cargo clippy`, `cargo fmt --check`).
- [ ] Tests added/updated and passing.
- [ ] No secrets, no debug prints (`console.log`, stray `dbg!`).
- [ ] Docs/README updated if behavior or options changed.
- [ ] CI is green.

## Architecture

Before large changes, skim the design docs in [`docs/`](docs/):
[ARCHITECTURE](docs/ARCHITECTURE.md), [DATA_MODEL](docs/DATA_MODEL.md),
[ROADMAP](docs/ROADMAP.md).

## Security

Found a vulnerability? **Do not open a public issue.** See
[`SECURITY.md`](SECURITY.md) for private disclosure.

## License

By contributing, you agree your contributions are licensed under the project's
[MIT License](LICENSE).
