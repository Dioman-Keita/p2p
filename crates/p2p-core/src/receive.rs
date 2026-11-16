use std::path::{Path, PathBuf};

use iroh::endpoint::Connection;
use iroh_blobs::{
    api::{blobs::ExportMode, blobs::ExportOptions, remote::GetProgressItem},
    format::collection::Collection,
    get::request::get_hash_seq_and_sizes,
    Hash, HashAndFormat,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    paths::{non_existing, safe_relative_path},
    Error, FileEntry, Node, Result, Ticket, TransferEvent, TransferId,
};

/// Upper bound on the size of the file list we accept to download before the user accepts.
const MAX_HASH_SEQ_BYTES: u64 = 32 * 1024 * 1024; // 1M files
const MAX_META_BYTES: u64 = 64 * 1024 * 1024;

/// What a sender proposes. Nothing is downloaded until [`Offer::accept`].
#[derive(Debug)]
pub struct Offer {
    node: Node,
    conn: Connection,
    hash: Hash,
    entries: Vec<(PathBuf, Hash)>,
    files: Vec<FileEntry>,
    total_bytes: u64,
}

impl Node {
    /// Connects to the sender and fetches the list of files with their (verified) sizes.
    pub async fn fetch_offer(&self, ticket: &Ticket) -> Result<Offer> {
        let ticket = &ticket.0;
        let conn = self
            .endpoint()
            .connect(ticket.addr().clone(), iroh_blobs::ALPN)
            .await
            .map_err(Error::network)?;
        let hash = ticket.hash();

        let (hash_seq, sizes) = get_hash_seq_and_sizes(&conn, &hash, MAX_HASH_SEQ_BYTES, None)
            .await
            .map_err(Error::network)?;
        let meta_size = *sizes.first().ok_or(Error::OfferTooLarge)?;
        if meta_size > MAX_META_BYTES {
            return Err(Error::OfferTooLarge);
        }
        let meta_hash = hash_seq.iter().next().ok_or(Error::OfferTooLarge)?;
        let store = &self.inner.store;
        for content in [HashAndFormat::raw(hash), HashAndFormat::raw(meta_hash)] {
            store
                .remote()
                .fetch(conn.clone(), content)
                .await
                .map_err(Error::network)?;
        }
        let collection = Collection::load(hash, store.as_ref())
            .await
            .map_err(Error::network)?;

        let mut entries = Vec::with_capacity(collection.len());
        let mut files = Vec::with_capacity(collection.len());
        for ((name, file_hash), size) in collection.iter().zip(sizes.iter().skip(1)) {
            entries.push((safe_relative_path(name)?, *file_hash));
            files.push(FileEntry {
                name: name.clone(),
                size: *size,
            });
        }
        let total_bytes = files.iter().map(|f| f.size).sum();
        Ok(Offer {
            node: self.clone(),
            conn,
            hash,
            entries,
            files,
            total_bytes,
        })
    }
}

impl Offer {
    pub fn files(&self) -> &[FileEntry] {
        &self.files
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn reject(self) {
        self.conn.close(0u32.into(), b"rejected");
    }

    /// Downloads everything, verifies it, then writes the files under `destination`
    /// (created if needed). Existing files are never overwritten.
    pub fn accept(self, destination: impl Into<PathBuf>) -> Transfer {
        let id = self.node.next_transfer_id();
        let (tx, events) = mpsc::channel(64);
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let destination = destination.into();
        tokio::spawn(async move {
            let event = tokio::select! {
                res = self.run(destination, &tx) => match res {
                    Ok(files) => TransferEvent::Completed { files },
                    Err(e) => TransferEvent::Failed { reason: e.to_string() },
                },
                () = token.cancelled() => {
                    self.conn.close(0u32.into(), b"cancelled");
                    TransferEvent::Cancelled
                }
            };
            let _ = tx.send(event).await;
        });
        Transfer { id, events, cancel }
    }

    async fn run(
        &self,
        destination: PathBuf,
        tx: &mpsc::Sender<TransferEvent>,
    ) -> Result<Vec<PathBuf>> {
        let store = &self.node.inner.store;
        let total_bytes = self.total_bytes;
        let _ = tx
            .send(TransferEvent::Progress {
                bytes_done: 0,
                total_bytes,
            })
            .await;

        let progress = store
            .remote()
            .fetch(self.conn.clone(), HashAndFormat::hash_seq(self.hash));
        let mut stream = std::pin::pin!(progress.stream());
        loop {
            match n0_future::StreamExt::next(&mut stream).await {
                Some(GetProgressItem::Progress(bytes)) => {
                    // `try_send`: progress is best effort, never slow the download down.
                    let _ = tx.try_send(TransferEvent::Progress {
                        bytes_done: bytes.min(total_bytes),
                        total_bytes,
                    });
                }
                Some(GetProgressItem::Done(_)) => break,
                Some(GetProgressItem::Error(e)) => return Err(Error::network(e)),
                None => return Err(Error::network("download stream ended unexpectedly")),
            }
        }
        let _ = tx
            .send(TransferEvent::Progress {
                bytes_done: total_bytes,
                total_bytes,
            })
            .await;
        let _ = tx.send(TransferEvent::Writing).await;
        self.write_files(&destination).await
    }

    async fn write_files(&self, destination: &Path) -> Result<Vec<PathBuf>> {
        tokio::fs::create_dir_all(destination).await?;
        let destination = std::path::absolute(destination)?;
        let store = &self.node.inner.store;
        let mut written = Vec::with_capacity(self.entries.len());
        for (relative, hash) in &self.entries {
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let target = non_existing(&target);
            store
                .export_with_opts(ExportOptions {
                    hash: *hash,
                    // Moves the data out of the store when possible: no second copy on disk.
                    mode: ExportMode::TryReference,
                    target: target.clone(),
                })
                .finish()
                .await
                .map_err(Error::storage)?;
            written.push(target);
        }
        Ok(written)
    }
}

/// Cancels a [`Transfer`] from anywhere. Cheap to clone.
#[derive(Debug, Clone)]
pub struct CancelHandle(CancellationToken);

impl CancelHandle {
    pub fn cancel(&self) {
        self.0.cancel();
    }
}

/// Handle on an accepted transfer.
#[derive(Debug)]
pub struct Transfer {
    id: TransferId,
    events: mpsc::Receiver<TransferEvent>,
    cancel: CancellationToken,
}

impl Transfer {
    pub fn id(&self) -> TransferId {
        self.id
    }

    /// Next lifecycle event; `None` after the terminal event has been returned.
    pub async fn next_event(&mut self) -> Option<TransferEvent> {
        self.events.recv().await
    }

    /// A handle that cancels this transfer; can be moved to another task.
    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(self.cancel.clone())
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Drives the transfer to its end and returns the written files.
    pub async fn wait(mut self) -> Result<Vec<PathBuf>> {
        while let Some(event) = self.next_event().await {
            match event {
                TransferEvent::Completed { files } => return Ok(files),
                TransferEvent::Failed { reason } => return Err(Error::Network(reason)),
                TransferEvent::Cancelled => return Err(Error::Cancelled),
                _ => {}
            }
        }
        Err(Error::network("transfer task ended without a result"))
    }
}
