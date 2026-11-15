use thiserror::Error;

/// Errors surfaced by the transfer engine.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid ticket: {0}")]
    InvalidTicket(String),
    #[error("nothing to share")]
    NothingToShare,
    #[error("two shared files would have the same name: {0}")]
    DuplicateName(String),
    #[error("the peer sent an unsafe file name: {0:?}")]
    UnsafeName(String),
    #[error("the offer is too large to be inspected")]
    OfferTooLarge,
    #[error("transfer cancelled")]
    Cancelled,
    #[error("network error: {0}")]
    Network(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub(crate) fn network(err: impl std::fmt::Display) -> Self {
        Self::Network(err.to_string())
    }

    pub(crate) fn storage(err: impl std::fmt::Display) -> Self {
        Self::Storage(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
