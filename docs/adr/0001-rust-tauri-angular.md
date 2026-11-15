# ADR 0001: Rust engine, Tauri 2 shell, Angular UI

- Status: Accepted
- Date: 2026-11-15

## Context

We need a desktop utility for Windows first, with a web-based UI (Angular), a high-performance
file transfer engine, and a path to macOS and Linux later.

## Decision

- **Engine in Rust.**
- **Tauri 2** (stable) as the desktop shell. It uses the system WebView2 on Windows.
- **Angular** for the UI.

## Alternatives considered

- **Go + Wails.** Viable, but its ecosystem for packaging, auto-update and signing is less mature,
  and the best P2P transport library for our needs (iroh, see ADR 0002) is Rust.
- **Electron.** Bundles Chromium and Node, so installers are over 100 MB and memory use is higher.
  Its security model is also weaker than Tauri's capability system.
- **Tauri 3.** Still in alpha. We will revisit it once it is stable.

## Consequences

- Small installers (about 5–10 MB), native performance, one language for the engine and the shell.
- The UI talks to Rust only through Tauri commands and events, which keeps the boundary explicit.
- Contributors need both the Rust and Node toolchains.
