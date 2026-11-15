# ADR 0003: Engine library + thin front-ends instead of DDD / FSD

- Status: Accepted
- Date: 2026-11-15

## Context

DDD and Feature-Sliced Design pay off in applications with a rich business domain and many features.
p2p is a utility: its complexity is technical (networking, file I/O), not business rules.

## Decision

- Backend: one engine crate (`p2p-core`) organised by technical responsibility (`node`, `ticket`, `send`, `receive`, `event`),
  and front-ends that only adapt it (CLI, Tauri).
- Frontend: a flat feature-folder Angular layout (`core`, `send`, `receive`, `shared`).

## Consequences

- Less indirection and fewer layers to navigate for a small codebase.
- The core can still be tested in isolation and reused (CLI, desktop, maybe a future service).
- If the product grows (accounts, teams, history sync), we will revisit this ADR.
