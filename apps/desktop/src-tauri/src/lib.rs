mod commands;
mod dto;
mod state;

use p2p_core::{Network, Node, NodeConfig};
use tauri::{ipc::Invoke, Manager, RunEvent, Runtime};

pub use crate::state::AppState;

/// All commands exposed to the UI. Public so integration tests can mount them on a mock app.
pub fn invoke_handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::share,
        commands::unshare,
        commands::fetch_offer,
        commands::reject_offer,
        commands::accept_offer,
        commands::cancel_transfer,
    ]
}

pub fn run() {
    let app = tauri::Builder::default()
        // Must be the first plugin: a second launch would otherwise start another node on the
        // same data directory and wipe the running instance's store. Focus the open window instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let node = tauri::async_runtime::block_on(Node::spawn(NodeConfig {
                data_dir,
                network: Network::Internet,
            }))?;
            app.manage(AppState::new(node));
            Ok(())
        })
        .invoke_handler(invoke_handler())
        .build(tauri::generate_context!())
        .expect("failed to build the application");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            let node = app.state::<AppState>().node.clone();
            let _ = tauri::async_runtime::block_on(node.shutdown());
        }
    });
}
