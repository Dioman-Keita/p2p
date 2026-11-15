use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use iroh_blobs::{
    api::blobs::{AddPathOptions, ImportMode},
    format::collection::Collection,
    hashseq::HashSeq,
    ticket::BlobTicket,
    BlobFormat,
};

use crate::{access::ShareEntry, Error, FileEntry, Node, Result, Ticket};

/// Content published by this node. Stays downloadable until [`Node::unshare`] or shutdown.
#[derive(Debug, Clone)]
pub struct Share {
    pub ticket: Ticket,
    pub files: Vec<FileEntry>,
    pub total_bytes: u64,
}

impl Node {
    /// Publishes files and folders (recursively) and returns the ticket to give to the receiver.
    ///
    /// Files are referenced in place, not copied: they must not be modified while shared.
    /// A folder keeps its name as the first path segment. Symbolic links and empty folders are skipped.
    pub async fn share(&self, paths: Vec<PathBuf>) -> Result<Share> {
        let files = tokio::task::spawn_blocking(move || collect_files(&paths))
            .await
            .map_err(Error::storage)??;
        if files.is_empty() {
            return Err(Error::NothingToShare);
        }

        let store = &self.inner.store;
        let mut tags = Vec::with_capacity(files.len() + 1);
        let mut collection = Collection::default();
        let mut entries = Vec::with_capacity(files.len());
        for (name, path, size) in files {
            let tag = store
                .add_path_with_opts(AddPathOptions {
                    path,
                    format: BlobFormat::Raw,
                    mode: ImportMode::TryReference,
                })
                .temp_tag()
                .await
                .map_err(Error::storage)?;
            collection.push(name.clone(), tag.hash());
            tags.push(tag);
            entries.push(FileEntry { name, size });
        }
        let root = collection.store(store).await.map_err(Error::storage)?;
        let hash = root.hash();
        // The root's hash sequence lists the metadata blob followed by every file.
        let hash_seq = store.get_bytes(hash).await.map_err(Error::storage)?;
        let members = HashSeq::try_from(hash_seq)
            .map_err(Error::storage)?
            .into_iter()
            .collect();
        tags.push(root);
        self.inner.shares.insert(
            hash,
            ShareEntry {
                _tags: tags,
                members,
            },
        );

        let ticket = Ticket(BlobTicket::new(
            self.addr().await,
            hash,
            BlobFormat::HashSeq,
        ));
        let total_bytes = entries.iter().map(|f| f.size).sum();
        Ok(Share {
            ticket,
            files: entries,
            total_bytes,
        })
    }

    /// Stops publishing a share. Transfers already in progress may fail.
    pub fn unshare(&self, ticket: &Ticket) {
        self.inner.shares.remove(&ticket.0.hash());
    }
}

/// Resolves the user's selection into `(name, absolute path, size)` triples.
fn collect_files(paths: &[PathBuf]) -> Result<Vec<(String, PathBuf, u64)>> {
    let mut out = Vec::new();
    for path in paths {
        let path = std::path::absolute(path)?;
        let meta = std::fs::symlink_metadata(&path)?;
        let root_name = file_name(&path)?;
        if meta.is_dir() {
            walk(&path, &root_name, &mut out)?;
        } else if meta.is_file() {
            out.push((root_name, path, meta.len()));
        }
    }
    let mut seen = HashSet::new();
    for (name, ..) in &out {
        if !seen.insert(name.as_str()) {
            return Err(Error::DuplicateName(name.clone()));
        }
    }
    Ok(out)
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf, u64)>) -> Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let meta = entry.metadata()?; // does not follow symlinks
        let name = format!("{prefix}/{}", file_name(&path)?);
        if meta.is_dir() {
            walk(&path, &name, out)?;
        } else if meta.is_file() {
            out.push((name, path, meta.len()));
        }
    }
    Ok(())
}

fn file_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unsupported file name: {}", path.display()),
            ))
        })
}
