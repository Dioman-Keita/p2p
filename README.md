# p2p

Share any file or folder directly between two PCs. No account, no cloud storage, no size limit.

> **Status:** early development. Windows is the first supported platform.

## How it works

1. The sender picks files and gets a **ticket** (short text or QR code).
2. The receiver pastes the ticket, reviews the offer, then accepts or rejects it.
3. Files go **directly** from one PC to the other over an encrypted QUIC connection.
   This works on the local network and across the internet (NAT traversal, with a relay fallback).
4. Every file is checked against its BLAKE3 hash when it arrives.

## Repository layout

```
crates/
  p2p-core/     Transfer engine (Rust library). The actual product.
  p2p-cli/      Thin command-line front-end (headless use, end-to-end tests).
apps/
  desktop/      Tauri 2 shell + Angular UI (thin front-end over p2p-core).
docs/
  architecture.md
  adr/          Architecture Decision Records
```

## Download

Every CI run on GitHub Actions builds the Windows installers (`.msi` and `.exe`).
Download them from the run's **p2p-windows-installers** artifact.

## Development

Prerequisites: Rust (stable), Node.js `^22.22.3 || ^24.15.0 || >=26.0.0` (required by Angular 22), and on Windows the WebView2 runtime (preinstalled on Windows 10/11).
Also install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS:
on Windows, Microsoft C++ Build Tools with the **Desktop development with C++** workload (includes the Windows SDK);
on Linux, the WebKitGTK system packages.

```bash
cd apps/desktop/ui && npm ci && npm run build && cd ../../..
cargo test --workspace
```

The Rust workspace embeds the built UI, so build it once before compiling.

### Desktop app

```bash
cd apps/desktop/ui
```
```bash
npm run tauri dev
```

`npm run tauri dev` starts the Angular dev server with hot reload and opens the app.
`npm run tauri build` produces the installers in `target/release/bundle/`.

UI tests run with `npm test` in `apps/desktop/ui`.

After changing a type in `apps/desktop/src-tauri/src/dto.rs`, run `cargo test -p p2p-desktop` to regenerate the
TypeScript bindings in `apps/desktop/ui/src/app/core/bindings/`. CI fails when they are out of date.

### Try it with the CLI

On the sending PC:

```bash
cargo run -p p2p-cli -- send <file-or-folder>...
```

It prints a ticket. On the receiving PC:

```bash
cargo run -p p2p-cli -- receive <ticket> --dest <folder>
```

Add `--direct 0.0.0.0:0` on both sides to disable relays and connect only over the local network.

See [docs/architecture.md](docs/architecture.md) for the design.
