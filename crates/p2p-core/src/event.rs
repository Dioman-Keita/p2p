use std::path::PathBuf;

/// Identifier of a transfer, unique within a running [`crate::Node`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransferId(pub u64);

/// A file that is part of a share or an offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Path relative to the shared root, always using `/` as separator.
    pub name: String,
    pub size: u64,
}

/// Lifecycle of an accepted transfer, as seen by the receiver.
///
/// A transfer always ends with exactly one of `Completed`, `Failed` or `Cancelled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferEvent {
    /// Verified payload bytes received so far.
    Progress {
        bytes_done: u64,
        total_bytes: u64,
    },
    /// All data is received and verified; files are being written to the destination.
    Writing,
    /// Files were written. Paths are absolute and may differ from the offered names
    /// when a file with the same name already existed.
    Completed {
        files: Vec<PathBuf>,
    },
    Failed {
        reason: String,
    },
    Cancelled,
}

impl TransferEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled
        )
    }
}
