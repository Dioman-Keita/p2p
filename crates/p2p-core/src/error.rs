use thiserror::Error;

/// Errors surfaced by the transfer engine.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid ticket: {0}")]
    InvalidTicket(String),
    #[error("transfer rejected by peer")]
    Rejected,
    #[error("transfer cancelled")]
    Cancelled,
    #[error("integrity check failed for {path}")]
    Integrity { path: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
