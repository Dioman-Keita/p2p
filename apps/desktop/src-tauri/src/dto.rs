//! Types exchanged with the UI. TypeScript definitions are generated from them by ts-rs
//! (`cargo test -p p2p-desktop` writes `ui/src/app/core/bindings/`).

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileEntry {
    pub name: String,
    #[ts(type = "number")]
    pub size: u64,
}

impl From<&p2p_core::FileEntry> for FileEntry {
    fn from(f: &p2p_core::FileEntry) -> Self {
        Self {
            name: f.name.clone(),
            size: f.size,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Share {
    pub ticket: String,
    pub files: Vec<FileEntry>,
    #[ts(type = "number")]
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Offer {
    #[ts(type = "number")]
    pub offer_id: u64,
    pub files: Vec<FileEntry>,
    #[ts(type = "number")]
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum TransferEvent {
    Progress {
        #[ts(type = "number")]
        bytes_done: u64,
        #[ts(type = "number")]
        total_bytes: u64,
    },
    Writing,
    Completed {
        files: Vec<String>,
    },
    Failed {
        reason: String,
    },
    Cancelled,
}

impl From<p2p_core::TransferEvent> for TransferEvent {
    fn from(e: p2p_core::TransferEvent) -> Self {
        use p2p_core::TransferEvent as E;
        match e {
            E::Progress {
                bytes_done,
                total_bytes,
            } => Self::Progress {
                bytes_done,
                total_bytes,
            },
            E::Writing => Self::Writing,
            E::Completed { files } => Self::Completed {
                files: files.iter().map(|p| p.display().to_string()).collect(),
            },
            E::Failed { reason } => Self::Failed { reason },
            E::Cancelled => Self::Cancelled,
        }
    }
}
