use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use p2p_core::{CancelHandle, Node, Offer};

/// Everything the commands need. Offers wait here between `fetch_offer` and `accept`/`reject`.
pub struct AppState {
    pub node: Node,
    offers: Mutex<HashMap<u64, Offer>>,
    next_offer: AtomicU64,
    transfers: Mutex<HashMap<u64, CancelHandle>>,
}

impl AppState {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            offers: Mutex::default(),
            next_offer: AtomicU64::new(1),
            transfers: Mutex::default(),
        }
    }

    pub fn park_offer(&self, offer: Offer) -> u64 {
        let id = self.next_offer.fetch_add(1, Ordering::Relaxed);
        self.offers.lock().expect("poisoned").insert(id, offer);
        id
    }

    pub fn take_offer(&self, id: u64) -> Option<Offer> {
        self.offers.lock().expect("poisoned").remove(&id)
    }

    pub fn track_transfer(&self, id: u64, cancel: CancelHandle) {
        self.transfers.lock().expect("poisoned").insert(id, cancel);
    }

    pub fn untrack_transfer(&self, id: u64) {
        self.transfers.lock().expect("poisoned").remove(&id);
    }

    pub fn cancel_transfer(&self, id: u64) {
        if let Some(handle) = self.transfers.lock().expect("poisoned").get(&id) {
            handle.cancel();
        }
    }
}
