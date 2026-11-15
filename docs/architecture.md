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
| `node`    | `Node`: starts/stops the iroh endpoint and the blob store. |
| `share`   | `Node::share`: imports files/folders (referenced in place, not copied) as a collection and returns a `Share` with its `Ticket`. |
| `receive` | `Node::fetch_offer` → `Offer` (file list with verified sizes) → `accept` → `Transfer` (events, cancel). |
| `access`  | Gatekeeper: the node only serves content of active shares. |
| `paths`   | Validates file names received from the network and avoids overwriting existing files. |
| `ticket`  | `Ticket`: the token exchanged between peers. |
| `event`   | `TransferEvent` and `FileEntry`, consumed by front-ends. |
| `error`   | The single `Error` type of the public API. |

### Transfer flow

```
Sender                                   Receiver
share(paths) ──► Ticket ── (copy/QR) ──► fetch_offer(ticket)
                                          │  downloads the file list + verified sizes only
                                          ▼
                                         Offer ──► reject()  (connection closed)
                                          │
                                          ▼ accept(dest)
serves blobs (if still shared) ◄──────── Transfer: Progress… → Writing → Completed | Failed | Cancelled
```

- Shared content lives in an `iroh-blobs` file store under `<data_dir>/store`, wiped on start.
  Shares therefore do not survive a restart. Sent files are referenced in place.
  Received data is moved out of the store into the destination when possible, so it does not take twice the disk space.
- Everything received is BLAKE3-verified before it is written to the destination.

### Security notes

- Connections are end-to-end encrypted and authenticated by the peers' public keys (QUIC / TLS 1.3).
- A ticket is a **bearer token**: anyone holding it can download the share while it is active.
  `Node::unshare` revokes it immediately.
- Only content of active shares is served: every request is checked against the share registry.
  Requests that fetch many arbitrary hashes in one go are disabled.
- File names from the peer are untrusted. Absolute paths, `..`, drive prefixes, Windows reserved
  names (`CON`, `NUL`…) and invalid characters are rejected. Files are never overwritten (`name (1).ext`).
- Before acceptance, the receiver downloads at most the file list, and its size is capped.

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
