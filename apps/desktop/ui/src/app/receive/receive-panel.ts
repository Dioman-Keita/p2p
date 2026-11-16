import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';

import type { Offer } from '../core/bindings/Offer';
import type { TransferEvent } from '../core/bindings/TransferEvent';
import { errorMessage, P2pService } from '../core/p2p.service';
import { FileSizePipe } from '../shared/file-size.pipe';

/** The receive flow is a small state machine; the template renders one state at a time. */
export type ReceiveState =
  | { step: 'idle' }
  | { step: 'connecting' }
  | { step: 'offer'; offer: Offer; destination: string | null }
  | {
      step: 'transferring';
      transferId: number | null;
      bytesDone: number;
      totalBytes: number;
      writing: boolean;
    }
  | { step: 'done'; files: string[] };

@Component({
  selector: 'app-receive-panel',
  imports: [FormsModule, FileSizePipe],
  templateUrl: './receive-panel.html',
})
export class ReceivePanel {
  private readonly p2p = inject(P2pService);

  protected ticket = '';
  /** Identifies the latest accept, so late responses or events of an older one are ignored. */
  private acceptSeq = 0;
  protected readonly state = signal<ReceiveState>({ step: 'idle' });
  protected readonly error = signal<string | null>(null);

  async connect(): Promise<void> {
    const ticket = this.ticket.trim();
    if (!ticket) return;
    this.error.set(null);
    this.state.set({ step: 'connecting' });
    try {
      const offer = await this.p2p.fetchOffer(ticket);
      this.state.set({ step: 'offer', offer, destination: null });
    } catch (err) {
      this.fail(err);
    }
  }

  async chooseDestination(): Promise<void> {
    const destination = await this.p2p.pickDestination();
    const state = this.state();
    if (destination && state.step === 'offer') {
      this.state.set({ ...state, destination });
    }
  }

  async reject(): Promise<void> {
    const state = this.state();
    if (state.step !== 'offer') return;
    await this.p2p.rejectOffer(state.offer.offerId);
    // Reset only if this offer is still displayed (a destination may have been picked meanwhile).
    // If Accept won the race, a transfer is now running and must not be discarded.
    const current = this.state();
    if (current.step === 'offer' && current.offer.offerId === state.offer.offerId) this.reset();
  }

  async accept(): Promise<void> {
    const state = this.state();
    if (state.step !== 'offer' || !state.destination) return;
    const seq = ++this.acceptSeq;
    this.state.set({
      step: 'transferring',
      transferId: null,
      bytesDone: 0,
      totalBytes: state.offer.totalBytes,
      writing: false,
    });
    try {
      const transferId = await this.p2p.acceptOffer(
        state.offer.offerId,
        state.destination,
        (event) => {
          if (seq === this.acceptSeq) this.onEvent(event);
        },
      );
      if (seq !== this.acceptSeq) return;
      this.state.update((s) => (s.step === 'transferring' ? { ...s, transferId } : s));
    } catch (err) {
      if (seq === this.acceptSeq) this.fail(err);
    }
  }

  async cancel(): Promise<void> {
    const state = this.state();
    if (state.step === 'transferring' && state.transferId !== null) {
      await this.p2p.cancelTransfer(state.transferId);
    }
  }

  reset(): void {
    this.ticket = '';
    this.state.set({ step: 'idle' });
  }

  protected percent(bytesDone: number, totalBytes: number): number {
    return totalBytes === 0 ? 100 : Math.floor((bytesDone / totalBytes) * 100);
  }

  private onEvent(event: TransferEvent): void {
    switch (event.kind) {
      case 'progress':
        this.state.update((s) =>
          s.step === 'transferring'
            ? { ...s, bytesDone: event.bytesDone, totalBytes: event.totalBytes }
            : s,
        );
        break;
      case 'writing':
        this.state.update((s) => (s.step === 'transferring' ? { ...s, writing: true } : s));
        break;
      case 'completed':
        this.state.set({ step: 'done', files: event.files });
        break;
      case 'failed':
        this.fail(event.reason);
        break;
      case 'cancelled':
        this.reset();
        this.error.set('Transfer cancelled.');
        break;
    }
  }

  private fail(err: unknown): void {
    this.state.set({ step: 'idle' });
    this.error.set(errorMessage(err));
  }
}
