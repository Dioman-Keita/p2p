# ADR 0002: iroh for transport

- Status: Accepted
- Date: 2026-11-15

## Context

Two PCs must connect directly, securely and reliably, on a LAN or across the internet behind NATs,
without us operating a storage server.

## Decision

Use [`iroh`](https://crates.io/crates/iroh) (QUIC connections addressed by public key, with hole punching
and relay fallback) and [`iroh-blobs`](https://crates.io/crates/iroh-blobs) (BLAKE3-verified,
resumable content transfer).

## Alternatives considered

- **Custom TCP/QUIC + mDNS.** LAN only unless we also build NAT traversal. It is a large,
  security-sensitive surface to own.
- **WebRTC.** Built for browsers, with heavy signalling requirements and awkward native Rust support.

## Consequences

- Encryption (TLS 1.3), NAT traversal and integrity checking come from the library, not from our code.
- By default, relayed traffic uses public relay servers. These only carry end-to-end encrypted traffic.
  We may self-host a relay later.
- iroh types stay inside `p2p-core` (see architecture principles), which limits the impact of upstream API changes.
