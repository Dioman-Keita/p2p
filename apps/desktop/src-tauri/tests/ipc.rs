//! Drives the real commands through Tauri's mock IPC, against two real nodes on loopback.

use std::{path::Path, time::Duration};

use p2p_core::{Network, Node, NodeConfig};
use p2p_desktop_lib::{invoke_handler, AppState};
use serde_json::{json, Value};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{get_ipc_response, mock_builder, MockRuntime, INVOKE_KEY},
    webview::InvokeRequest,
    WebviewWindow, WebviewWindowBuilder,
};

fn spawn_node(rt: &tokio::runtime::Runtime, dir: &Path) -> Node {
    rt.block_on(Node::spawn(NodeConfig {
        data_dir: dir.to_owned(),
        network: Network::Direct {
            bind: "127.0.0.1:0".parse().unwrap(),
        },
    }))
    .unwrap()
}

fn app_window(node: Node) -> WebviewWindow<MockRuntime> {
    let app = mock_builder()
        .manage(AppState::new(node))
        .invoke_handler(invoke_handler())
        // The real config and capabilities, so permissions are checked as in the app.
        .build(tauri::generate_context!("tauri.conf.json", test = true))
        .unwrap();
    WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap()
}

/// Origin of the app's own pages: permissions only grant app commands to it.
#[cfg(windows)]
const APP_ORIGIN: &str = "http://tauri.localhost";
#[cfg(not(windows))]
const APP_ORIGIN: &str = "tauri://localhost";

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: APP_ORIGIN.parse().unwrap(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn share_fetch_accept_through_ipc() {
    // The app's own runtime is Tauri's; this one only hosts the sender, like another PC would.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("hello.txt");
    std::fs::write(&src, b"hello from the other PC").unwrap();

    let sender = app_window(spawn_node(&rt, &tmp.path().join("sender")));
    let receiver = app_window(spawn_node(&rt, &tmp.path().join("receiver")));

    let share = invoke(&sender, "share", json!({ "paths": [src] })).unwrap();
    assert_eq!(share["files"][0]["name"], "hello.txt");

    let offer = invoke(
        &receiver,
        "fetch_offer",
        json!({ "ticket": share["ticket"] }),
    )
    .unwrap();
    assert_eq!(offer["totalBytes"], 23);

    let dest = tmp.path().join("dest");
    let transfer_id = invoke(
        &receiver,
        "accept_offer",
        json!({ "offerId": offer["offerId"], "destination": dest, "onEvent": "__CHANNEL__:1" }),
    )
    .unwrap();
    assert!(transfer_id.as_u64().is_some());

    let received = dest.join("hello.txt");
    for _ in 0..100 {
        if received.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(std::fs::read(received).unwrap(), b"hello from the other PC");

    // The offer was consumed by accept.
    let again = invoke(
        &receiver,
        "accept_offer",
        json!({ "offerId": offer["offerId"], "destination": dest, "onEvent": "__CHANNEL__:2" }),
    );
    assert!(again.is_err());
}

#[test]
fn invalid_ticket_is_reported_as_a_message() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let window = app_window(spawn_node(&rt, tmp.path()));
    let err = invoke(&window, "fetch_offer", json!({ "ticket": "nope" })).unwrap_err();
    let msg = err.as_str().unwrap();
    assert!(msg.starts_with("invalid ticket"), "unexpected error: {msg}");
}
