//! Serve only what is currently shared.
//!
//! The blobs protocol would otherwise answer any request for any content in the store.
//! Every incoming request is checked against the registry of active shares.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use iroh_blobs::{
    api::TempTag,
    provider::events::{AbortReason, EventMask, EventSender, ProviderMessage, RequestMode},
    Hash,
};

/// One active share: the tags that keep its content in the store, and every hash it contains.
pub(crate) struct ShareEntry {
    pub(crate) _tags: Vec<TempTag>,
    pub(crate) members: HashSet<Hash>,
}

#[derive(Clone, Default)]
pub(crate) struct Shares(Arc<Mutex<HashMap<Hash, ShareEntry>>>);

impl Shares {
    pub(crate) fn insert(&self, root: Hash, entry: ShareEntry) {
        self.0.lock().expect("poisoned").insert(root, entry);
    }

    pub(crate) fn remove(&self, root: &Hash) {
        self.0.lock().expect("poisoned").remove(root);
    }

    pub(crate) fn clear(&self) {
        self.0.lock().expect("poisoned").clear();
    }

    /// A whole share may be requested by its root; its parts only one blob at a time.
    fn allows(&self, hash: &Hash, single_blob: bool) -> bool {
        let shares = self.0.lock().expect("poisoned");
        shares.contains_key(hash)
            || (single_blob && shares.values().any(|s| s.members.contains(hash)))
    }

    /// Builds the event sender to give to the blobs protocol.
    pub(crate) fn gatekeeper(&self) -> EventSender {
        let mask = EventMask {
            get: RequestMode::Intercept,
            // Could request arbitrary hashes in one go: not needed by our receivers.
            get_many: RequestMode::Disabled,
            ..EventMask::DEFAULT
        };
        let (sender, mut rx) = EventSender::channel(32, mask);
        let shares = self.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let ProviderMessage::GetRequestReceived(msg) = msg {
                    let request = &msg.request;
                    let res = if shares.allows(&request.hash, request.ranges.is_blob()) {
                        Ok(())
                    } else {
                        Err(AbortReason::Permission)
                    };
                    msg.tx.send(res).await.ok();
                }
            }
        });
        sender
    }
}
