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

## Development

Prerequisites: Rust (stable), Node.js 22+, and on Windows the WebView2 runtime (preinstalled on Windows 10/11).

```bash
cargo test --workspace
```

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
