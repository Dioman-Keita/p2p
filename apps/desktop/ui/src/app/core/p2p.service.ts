import { Injectable } from '@angular/core';
import { Channel, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

import type { Offer } from './bindings/Offer';
import type { Share } from './bindings/Share';
import type { TransferEvent } from './bindings/TransferEvent';

/**
 * Typed bridge to the Rust side. Components only talk to the engine through this service,
 * which keeps them testable with a fake.
 */
@Injectable({ providedIn: 'root' })
export class P2pService {
  share(paths: string[]): Promise<Share> {
    return invoke('share', { paths });
  }

  unshare(ticket: string): Promise<void> {
    return invoke('unshare', { ticket });
  }

  fetchOffer(ticket: string): Promise<Offer> {
    return invoke('fetch_offer', { ticket });
  }

  rejectOffer(offerId: number): Promise<void> {
    return invoke('reject_offer', { offerId });
  }

  /** Starts the download; `onEvent` receives every event until a terminal one. */
  acceptOffer(
    offerId: number,
    destination: string,
    onEvent: (event: TransferEvent) => void,
  ): Promise<number> {
    const channel = new Channel<TransferEvent>();
    channel.onmessage = onEvent;
    return invoke('accept_offer', { offerId, destination, onEvent: channel });
  }

  cancelTransfer(transferId: number): Promise<void> {
    return invoke('cancel_transfer', { transferId });
  }

  async pickFiles(): Promise<string[]> {
    return toArray(await open({ multiple: true, directory: false, title: 'Files to share' }));
  }

  async pickFolders(): Promise<string[]> {
    return toArray(await open({ multiple: true, directory: true, title: 'Folders to share' }));
  }

  async pickDestination(): Promise<string | null> {
    const picked = await open({ multiple: false, directory: true, title: 'Save files to' });
    return toArray(picked)[0] ?? null;
  }
}

function toArray(picked: string | string[] | null): string[] {
  if (picked === null) return [];
  return Array.isArray(picked) ? picked : [picked];
}

/** Errors from `invoke` are plain strings; anything else is unexpected. */
export function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  return 'Unexpected error';
}
