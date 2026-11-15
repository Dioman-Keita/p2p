use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use iroh::{endpoint::presets, protocol::Router, Endpoint, RelayMode};
use iroh_blobs::{store::fs::FsStore, BlobsProtocol};

use crate::{access::Shares, Error, Result, TransferId};

/// How the node reaches peers.
#[derive(Debug, Clone, Default)]
pub enum Network {
    /// Direct connections when possible, NAT traversal and relay fallback otherwise.
    /// Works on the LAN and across the internet.
    #[default]
    Internet,
    /// Direct connections only, on the given local address (no relay, no address lookup).
    /// The ticket must carry an address the receiver can reach. Mostly useful for tests.
    Direct { bind: SocketAddr },
}

#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Directory for the node's working data. Its `store` sub-directory is wiped on start:
    /// shares do not survive a restart.
    pub data_dir: PathBuf,
    pub network: Network,
}

/// A running peer. Cheap to clone; all clones share the same node.
#[derive(Clone)]
pub struct Node {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    pub(crate) router: Router,
    pub(crate) store: FsStore,
    store_dir: PathBuf,
    /// Active shares: keeps their content in the store and decides what may be served.
    pub(crate) shares: Shares,
    next_transfer: AtomicU64,
    wait_for_relay: bool,
}

impl Node {
    pub async fn spawn(config: NodeConfig) -> Result<Self> {
        let store_dir = config.data_dir.join("store");
        if store_dir.exists() {
            tokio::fs::remove_dir_all(&store_dir).await?;
        }
        tokio::fs::create_dir_all(&store_dir).await?;
        let store = FsStore::load(&store_dir).await.map_err(Error::storage)?;

        let (endpoint, wait_for_relay) = match config.network {
            Network::Internet => (Endpoint::bind(presets::N0).await, true),
            Network::Direct { bind } => {
                let builder = Endpoint::builder(presets::Minimal)
                    .relay_mode(RelayMode::Disabled)
                    .clear_ip_transports()
                    .bind_addr(bind)
                    .map_err(Error::network)?;
                (builder.bind().await, false)
            }
        };
        let endpoint = endpoint.map_err(Error::network)?;

        let shares = Shares::default();
        let blobs = BlobsProtocol::new(&store, Some(shares.gatekeeper()));
        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, blobs)
            .spawn();

        Ok(Self {
            inner: Arc::new(Inner {
                router,
                store,
                store_dir,
                shares,
                next_transfer: AtomicU64::new(1),
                wait_for_relay,
            }),
        })
    }

    pub(crate) fn endpoint(&self) -> &Endpoint {
        self.inner.router.endpoint()
    }

    /// The current address of this node, as advertised in tickets.
    pub(crate) async fn addr(&self) -> iroh::EndpointAddr {
        if self.inner.wait_for_relay {
            // Wait (bounded) until a home relay is known, so tickets work across NATs.
            // Without a relay we still have direct addresses, which is enough on a LAN.
            let _ = tokio::time::timeout(Duration::from_secs(10), self.endpoint().online()).await;
        }
        self.endpoint().addr()
    }

    pub(crate) fn next_transfer_id(&self) -> TransferId {
        TransferId(self.inner.next_transfer.fetch_add(1, Ordering::Relaxed))
    }

    /// Stops serving and releases resources. Working data on disk is deleted.
    pub async fn shutdown(self) -> Result<()> {
        self.inner.shares.clear();
        // Also shuts the store down (the blobs protocol owns it).
        self.inner.router.shutdown().await.map_err(Error::network)?;
        let _ = tokio::fs::remove_dir_all(&self.inner.store_dir).await;
        Ok(())
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.endpoint().id())
            .finish_non_exhaustive()
    }
}
