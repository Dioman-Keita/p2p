use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use p2p_core::{CancelHandle, Node, Offer};

/// Everything the commands need. Offers wait here between `fetch_offer` and `accept`/`reject`.
pub struct AppState {
    pub(crate) node: Node,
    offers: Mutex<HashMap<u64, Offer>>,
    next_offer: AtomicU64,
    transfers: Transfers,
}

impl AppState {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            offers: Mutex::default(),
            next_offer: AtomicU64::new(1),
            transfers: Transfers::default(),
        }
    }

    pub(crate) fn park_offer(&self, offer: Offer) -> u64 {
        let id = self.next_offer.fetch_add(1, Ordering::Relaxed);
        self.offers.lock().expect("poisoned").insert(id, offer);
        id
    }

    pub(crate) fn take_offer(&self, id: u64) -> Option<Offer> {
        self.offers.lock().expect("poisoned").remove(&id)
    }

    pub(crate) fn transfers(&self) -> Transfers {
        self.transfers.clone()
    }
}

/// Cancel handles of running transfers. Cheap to clone into the task that follows a transfer.
#[derive(Clone, Default)]
pub(crate) struct Transfers(Arc<Mutex<HashMap<u64, CancelHandle>>>);

impl Transfers {
    pub(crate) fn track(&self, id: u64, cancel: CancelHandle) {
        self.0.lock().expect("poisoned").insert(id, cancel);
    }

    pub(crate) fn untrack(&self, id: u64) {
        self.0.lock().expect("poisoned").remove(&id);
    }

    pub(crate) fn cancel(&self, id: u64) {
        if let Some(handle) = self.0.lock().expect("poisoned").get(&id) {
            handle.cancel();
        }
    }
}
