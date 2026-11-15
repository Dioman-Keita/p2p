mod commands;
mod dto;
mod state;

use p2p_core::{Network, Node, NodeConfig};
use tauri::{Manager, RunEvent};

use crate::state::AppState;

pub fn run() {
    let app = tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![
            commands::share,
            commands::unshare,
            commands::fetch_offer,
            commands::reject_offer,
            commands::accept_offer,
            commands::cancel_transfer,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build the application");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            let node = app.state::<AppState>().node.clone();
            let _ = tauri::async_runtime::block_on(node.shutdown());
        }
    });
}
