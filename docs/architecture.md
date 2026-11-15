# Architecture

p2p is a **utility**, not a business application. There is no rich domain to model,
so the architecture stays deliberately small: one engine library and thin front-ends.

## Principles

- **The engine is the product.** All networking, pairing, and file I/O live in `p2p-core`.
  It has no knowledge of any UI.
- **Front-ends are thin.** The CLI and the desktop app only translate user input into engine
  calls and render `TransferEvent`s. No transfer logic in front-ends.
- **One-way dependencies.** `p2p-cli` → `p2p-core` ← `apps/desktop/src-tauri`. The core never depends on a front-end.
- **Event-driven progress.** The engine emits a stream of `TransferEvent`s
  (`Offered`, `Started`, `Progress`, `Completed`, `Failed`). The UI is a projection of that stream.
- **Wrap third-party types.** Types from `iroh` do not leak through the public API of `p2p-core`
  (for example, `Ticket` wraps the underlying address/hash format). The transport can change without breaking front-ends.

## Components

```
┌─────────────────────────┐     ┌──────────────┐
│ apps/desktop            │     │ p2p-cli      │
│  ui/ (Angular)          │     │              │
│   │ invoke / events     │     │              │
│  src-tauri/ (Rust)      │     │              │
└───────────┬─────────────┘     └──────┬───────┘
            │                          │
            └──────────┬───────────────┘
                       ▼
              ┌──────────────────┐
              │ p2p-core         │
              │  node     (iroh endpoint lifecycle)
              │  ticket   (sharing token)
              │  send / receive  │
              │  event    (TransferEvent stream)
              └────────┬─────────┘
                       ▼
                iroh + iroh-blobs (QUIC, NAT traversal, BLAKE3)
```

### `p2p-core` modules

| Module    | Responsibility |
|-----------|----------------|
| `node`    | Start/stop the iroh endpoint; holds the node's key pair. |
| `ticket`  | Encode/decode the token exchanged between peers. |
| `send`    | Import files/folders, publish them, produce a `Ticket`. |
| `receive` | Connect with a `Ticket`, present the offer, download and verify. |
| `event`   | `TransferEvent` and related types consumed by front-ends. |
| `error`   | The single `Error` type of the public API. |

### Desktop app (`apps/desktop`)

- `src-tauri/`: Tauri commands (`share`, `receive`, `accept`, `reject`, `cancel`) that call `p2p-core`
  and forward `TransferEvent`s to the UI as Tauri events. No logic beyond mapping.
- `ui/`: Angular (standalone components, signals). It uses a flat, feature-based layout
  and not a layered methodology such as FSD, because the app has only a few screens:

```
ui/src/app/
  core/       TauriBridge service: typed wrappers over invoke() and event listeners
  send/       pick files, show ticket / QR, progress
  receive/    paste ticket, review offer, accept, progress
  shared/     reusable presentational components and pipes (file size, ETA)
```

TypeScript types for commands and events are generated from the Rust types, so the two sides cannot drift apart.

## Scope

**v1:** Windows, one-to-one transfers of files and folders, accept/reject, progress, cancel, integrity check, MSI installer.

**Later:** resume interrupted transfers, transfer history, an Explorer "Send with p2p" context menu, macOS and Linux.
