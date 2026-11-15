//! Tauri commands: translate UI calls into `p2p-core` calls. No transfer logic here.

use std::path::PathBuf;

use p2p_core::Ticket;
use tauri::{ipc::Channel, AppHandle, Manager, State};

use crate::{dto, state::AppState};

/// Errors reach the UI as a plain message.
type CmdResult<T> = Result<T, String>;

fn msg(e: p2p_core::Error) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn share(state: State<'_, AppState>, paths: Vec<PathBuf>) -> CmdResult<dto::Share> {
    let share = state.node.share(paths).await.map_err(msg)?;
    Ok(dto::Share {
        ticket: share.ticket.to_string(),
        files: share.files.iter().map(Into::into).collect(),
        total_bytes: share.total_bytes,
    })
}

#[tauri::command]
pub fn unshare(state: State<'_, AppState>, ticket: String) -> CmdResult<()> {
    let ticket: Ticket = ticket.parse().map_err(msg)?;
    state.node.unshare(&ticket);
    Ok(())
}

#[tauri::command]
pub async fn fetch_offer(state: State<'_, AppState>, ticket: String) -> CmdResult<dto::Offer> {
    let ticket: Ticket = ticket.parse().map_err(msg)?;
    let offer = state.node.fetch_offer(&ticket).await.map_err(msg)?;
    let files = offer.files().iter().map(Into::into).collect();
    let total_bytes = offer.total_bytes();
    let offer_id = state.park_offer(offer);
    Ok(dto::Offer {
        offer_id,
        files,
        total_bytes,
    })
}

#[tauri::command]
pub fn reject_offer(state: State<'_, AppState>, offer_id: u64) {
    if let Some(offer) = state.take_offer(offer_id) {
        offer.reject();
    }
}

/// Starts the download and streams its events on `on_event`. Returns the transfer id.
#[tauri::command]
pub fn accept_offer(
    app: AppHandle,
    state: State<'_, AppState>,
    offer_id: u64,
    destination: PathBuf,
    on_event: Channel<dto::TransferEvent>,
) -> CmdResult<u64> {
    let offer = state
        .take_offer(offer_id)
        .ok_or_else(|| "this offer is no longer available".to_owned())?;
    let mut transfer = offer.accept(destination);
    let id = transfer.id().0;
    state.track_transfer(id, transfer.cancel_handle());
    tauri::async_runtime::spawn(async move {
        while let Some(event) = transfer.next_event().await {
            // The window may be gone; the transfer still runs to its end.
            let _ = on_event.send(event.into());
        }
        app.state::<AppState>().untrack_transfer(id);
    });
    Ok(id)
}

#[tauri::command]
pub fn cancel_transfer(state: State<'_, AppState>, transfer_id: u64) {
    state.cancel_transfer(transfer_id);
}
