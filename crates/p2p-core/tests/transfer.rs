//! End-to-end transfers between two nodes on the loopback interface.

use std::{fs, path::Path};

use p2p_core::{Error, Network, Node, NodeConfig, TransferEvent};

async fn node(dir: &Path) -> Node {
    Node::spawn(NodeConfig {
        data_dir: dir.to_owned(),
        network: Network::Direct {
            bind: "127.0.0.1:0".parse().unwrap(),
        },
    })
    .await
    .unwrap()
}

/// Large enough to span many BLAKE3 chunk groups, not a multiple of the chunk size.
fn pseudo_random(len: usize) -> Vec<u8> {
    let mut x: u32 = 0x1234_5678;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

#[tokio::test]
async fn transfers_files_and_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("album/sub")).unwrap();
    let big = pseudo_random(3 * 1024 * 1024 + 17);
    fs::write(src.join("big.bin"), &big).unwrap();
    fs::write(src.join("album/a.txt"), b"hello").unwrap();
    fs::write(src.join("album/sub/empty.txt"), b"").unwrap();

    let sender = node(&tmp.path().join("sender")).await;
    let receiver = node(&tmp.path().join("receiver")).await;

    let share = sender
        .share(vec![src.join("big.bin"), src.join("album")])
        .await
        .unwrap();
    assert_eq!(share.files.len(), 3);

    let ticket = share.ticket.to_string().parse().unwrap();
    let offer = receiver.fetch_offer(&ticket).await.unwrap();
    let mut names: Vec<_> = offer
        .files()
        .iter()
        .map(|f| (f.name.as_str(), f.size))
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            ("album/a.txt", 5),
            ("album/sub/empty.txt", 0),
            ("big.bin", big.len() as u64)
        ]
    );
    assert_eq!(offer.total_bytes(), share.total_bytes);

    let dest = tmp.path().join("dest");
    let mut transfer = offer.accept(&dest);
    let mut events = Vec::new();
    while let Some(event) = transfer.next_event().await {
        events.push(event);
    }
    let Some(TransferEvent::Completed { files }) = events.last() else {
        panic!("unexpected end: {events:?}");
    };
    assert_eq!(files.len(), 3);
    assert!(events.contains(&TransferEvent::Writing));

    assert_eq!(fs::read(dest.join("big.bin")).unwrap(), big);
    assert_eq!(fs::read(dest.join("album/a.txt")).unwrap(), b"hello");
    assert_eq!(fs::read(dest.join("album/sub/empty.txt")).unwrap(), b"");
    // The source is untouched.
    assert_eq!(fs::read(src.join("big.bin")).unwrap(), big);

    receiver.shutdown().await.unwrap();
    sender.shutdown().await.unwrap();
}

#[tokio::test]
async fn never_overwrites_existing_files() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("report.txt");
    fs::write(&src, b"new").unwrap();
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("report.txt"), b"old").unwrap();

    let sender = node(&tmp.path().join("sender")).await;
    let receiver = node(&tmp.path().join("receiver")).await;
    let share = sender.share(vec![src]).await.unwrap();

    let files = receiver
        .fetch_offer(&share.ticket)
        .await
        .unwrap()
        .accept(&dest)
        .wait()
        .await
        .unwrap();

    assert_eq!(files, [dest.join("report (1).txt")]);
    assert_eq!(fs::read(dest.join("report.txt")).unwrap(), b"old");
    assert_eq!(fs::read(dest.join("report (1).txt")).unwrap(), b"new");
}

#[tokio::test]
async fn cancelled_transfer_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("big.bin");
    fs::write(&src, pseudo_random(8 * 1024 * 1024)).unwrap();

    let sender = node(&tmp.path().join("sender")).await;
    let receiver = node(&tmp.path().join("receiver")).await;
    let share = sender.share(vec![src]).await.unwrap();

    let dest = tmp.path().join("dest");
    let transfer = receiver
        .fetch_offer(&share.ticket)
        .await
        .unwrap()
        .accept(&dest);
    transfer.cancel();
    assert!(matches!(transfer.wait().await, Err(Error::Cancelled)));
    assert!(!dest.join("big.bin").exists());
}

#[tokio::test]
async fn unshared_content_is_no_longer_offered() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("a.txt");
    fs::write(&src, b"a").unwrap();

    let sender = node(&tmp.path().join("sender")).await;
    let receiver = node(&tmp.path().join("receiver")).await;
    let share = sender.share(vec![src]).await.unwrap();
    // Still shared: the offer is visible.
    receiver.fetch_offer(&share.ticket).await.unwrap().reject();

    // Unshared while the sender keeps running: nothing is served any more.
    sender.unshare(&share.ticket);
    assert!(receiver.fetch_offer(&share.ticket).await.is_err());
}

#[tokio::test]
async fn rejects_empty_selection() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("empty")).unwrap();
    let sender = node(&tmp.path().join("sender")).await;
    assert!(matches!(
        sender.share(vec![tmp.path().join("empty")]).await,
        Err(Error::NothingToShare)
    ));
}
