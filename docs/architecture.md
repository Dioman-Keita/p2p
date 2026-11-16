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

**`src-tauri/`** (Rust) is a thin adapter over `p2p-core`. It contains no transfer logic.

| File          | Role |
|---------------|------|
| `lib.rs`      | Starts the `Node` (data in the OS app-local data dir) and registers the commands. Shuts the node down on exit. |
| `commands.rs` | `share`, `unshare`, `fetch_offer`, `reject_offer`, `accept_offer`, `cancel_transfer`. |
| `state.rs`    | Pending offers (between `fetch_offer` and accept/reject) and the cancel handles of running transfers. |
| `dto.rs`      | Serializable types sent to the UI. TypeScript definitions are generated from them. |

Transfer events reach the UI through a Tauri `Channel` passed to `accept_offer`, which is ordered and scoped to one transfer.
Errors reach the UI as plain messages.

**`ui/`** (Angular, standalone components, signals, zoneless) uses a flat, feature-based layout.
It does not use a layered methodology such as FSD, because the app has only a few screens:

```
ui/src/app/
  core/
    p2p.service.ts   typed wrapper over invoke(), channels and file dialogs; the only place that knows Tauri
    bindings/        generated from src-tauri/src/dto.rs (do not edit)
  send/              pick files and folders, share, show and copy the ticket, stop sharing
  receive/           paste ticket → review offer → choose folder → accept/reject → progress → done
  shared/            fileSize pipe
```

Components depend only on `P2pService`. Their tests swap it for a fake, so they run without Tauri.
The receive flow is a discriminated-union state (`idle | connecting | offer | transferring | done`) held in one signal.

TypeScript types are generated from the Rust DTOs with `ts-rs`, and CI checks they are up to date, so the two sides cannot drift apart.

## Scope

**v1:** Windows, one-to-one transfers of files and folders, accept/reject, progress, cancel, integrity check, MSI installer.

**Later:** resume interrupted transfers, transfer history, an Explorer "Send with p2p" context menu, macOS and Linux.
