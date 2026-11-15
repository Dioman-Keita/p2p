//! `p2p-core` is the transfer engine. It knows nothing about any UI.
//!
//! Front-ends (the CLI, the Tauri desktop app) talk to it only through
//! the public API re-exported here, and observe progress through
//! [`TransferEvent`]s.

pub mod error;
pub mod event;
pub mod ticket;

pub use error::{Error, Result};
pub use event::{TransferEvent, TransferId};
pub use ticket::Ticket;
