//! `p2p-core` is the transfer engine. It knows nothing about any UI.
//!
//! A typical session:
//!
//! - **Sender:** [`Node::spawn`], then [`Node::share`] some paths. Give the resulting
//!   [`Ticket`] to the receiver and keep the node running while they download.
//! - **Receiver:** [`Node::spawn`], then [`Node::fetch_offer`] with the ticket. Inspect the
//!   [`Offer`], then [`Offer::accept`] it into a destination directory (or [`Offer::reject`] it),
//!   and follow the returned [`Transfer`] through its [`TransferEvent`]s.
//!
//! Transport, encryption, NAT traversal and BLAKE3 verification are provided by `iroh` and
//! `iroh-blobs`. Their types are not part of this crate's public API.

mod access;
mod error;
mod event;
mod node;
mod paths;
mod receive;
mod share;
mod ticket;

pub use error::{Error, Result};
pub use event::{FileEntry, TransferEvent, TransferId};
pub use node::{Network, Node, NodeConfig};
pub use receive::{Offer, Transfer};
pub use share::Share;
pub use ticket::Ticket;
