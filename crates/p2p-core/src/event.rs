use std::path::PathBuf;

/// Opaque identifier of a transfer, unique within a running node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransferId(pub u64);

/// Everything a front-end needs to render a transfer's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferEvent {
    /// A peer offers files; the receiver must accept or reject.
    Offered {
        id: TransferId,
        files: Vec<FileEntry>,
        total_bytes: u64,
    },
    Started {
        id: TransferId,
    },
    Progress {
        id: TransferId,
        bytes_done: u64,
        total_bytes: u64,
    },
    Completed {
        id: TransferId,
        destination: Option<PathBuf>,
    },
    Failed {
        id: TransferId,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Path relative to the shared root, using `/` separators.
    pub relative_path: String,
    pub size: u64,
}
