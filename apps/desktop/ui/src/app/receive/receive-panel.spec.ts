import { TestBed } from '@angular/core/testing';

import type { TransferEvent } from '../core/bindings/TransferEvent';
import { P2pService } from '../core/p2p.service';
import { ReceivePanel } from './receive-panel';

describe('ReceivePanel', () => {
  let emit: (event: TransferEvent) => void = () => {};

  function setup(overrides: Partial<P2pService> = {}) {
    const p2p = {
      fetchOffer: vi.fn(async () => ({
        offerId: 7,
        files: [{ name: 'album/a.jpg', size: 2048 }],
        totalBytes: 2048,
      })),
      pickDestination: vi.fn(async () => 'D:\\Downloads'),
      rejectOffer: vi.fn(async () => {}),
      acceptOffer: vi.fn(
        async (_id: number, _dest: string, onEvent: (e: TransferEvent) => void) => {
          emit = onEvent;
          return 42;
        },
      ),
      cancelTransfer: vi.fn(async () => {}),
      ...overrides,
    };
    TestBed.configureTestingModule({ providers: [{ provide: P2pService, useValue: p2p }] });
    const fixture = TestBed.createComponent(ReceivePanel);
    const panel = fixture.componentInstance;
    (panel as unknown as { ticket: string }).ticket = '  blobTICKET ';
    return { fixture, panel, p2p, el: fixture.nativeElement as HTMLElement };
  }

  async function toOffer() {
    const ctx = setup();
    await ctx.panel.connect();
    await ctx.panel.chooseDestination();
    await ctx.fixture.whenStable();
    return ctx;
  }

  it('fetches the offer with the trimmed ticket and lists files', async () => {
    const { el, p2p } = await toOffer();
    expect(p2p.fetchOffer).toHaveBeenCalledWith('blobTICKET');
    expect(el.textContent).toContain('album/a.jpg');
    expect(el.textContent).toContain('D:\\Downloads');
  });

  it('accepts, follows progress and shows written files', async () => {
    const { panel, fixture, el, p2p } = await toOffer();
    await panel.accept();
    expect(p2p.acceptOffer).toHaveBeenCalledWith(7, 'D:\\Downloads', expect.any(Function));

    emit({ kind: 'progress', bytesDone: 1024, totalBytes: 2048 });
    await fixture.whenStable();
    expect(el.textContent).toContain('50%');

    await panel.cancel();
    expect(p2p.cancelTransfer).toHaveBeenCalledWith(42);

    emit({ kind: 'completed', files: ['D:\\Downloads\\album\\a.jpg'] });
    await fixture.whenStable();
    expect(el.textContent).toContain('D:\\Downloads\\album\\a.jpg');
  });

  it('rejects the offer and goes back to the ticket input', async () => {
    const { panel, fixture, el, p2p } = await toOffer();
    await panel.reject();
    await fixture.whenStable();
    expect(p2p.rejectOffer).toHaveBeenCalledWith(7);
    expect(el.querySelector('textarea')).not.toBeNull();
  });

  it('reports a failed transfer', async () => {
    const { panel, fixture, el } = await toOffer();
    await panel.accept();
    emit({ kind: 'failed', reason: 'network error: connection lost' });
    await fixture.whenStable();
    expect(el.querySelector('[role=alert]')?.textContent).toContain('connection lost');
  });

  it('reports connection errors', async () => {
    const { panel, fixture, el } = setup({
      fetchOffer: vi.fn(async () => Promise.reject('invalid ticket: x')),
    });
    await panel.connect();
    await fixture.whenStable();
    expect(el.querySelector('[role=alert]')?.textContent).toContain('invalid ticket');
  });

  it('keeps a running transfer when a pending reject resolves late', async () => {
    let finishReject: () => void = () => {};
    const { panel, fixture, el } = setup({
      rejectOffer: vi.fn(() => new Promise<void>((resolve) => (finishReject = resolve))),
    });
    await panel.connect();
    await panel.chooseDestination();
    const rejecting = panel.reject();
    await panel.accept();
    finishReject();
    await rejecting;
    await fixture.whenStable();
    expect(el.textContent).toContain('Receiving');
  });

  it('ignores the late response and events of a previous accept', async () => {
    const pending: Array<{ resolve: (id: number) => void; onEvent: (e: TransferEvent) => void }> =
      [];
    const { panel, fixture, el, p2p } = setup({
      acceptOffer: vi.fn(
        (_id: number, _dest: string, onEvent: (e: TransferEvent) => void) =>
          new Promise<number>((resolve) => pending.push({ resolve, onEvent })),
      ),
    });
    // First accept stays pending; its transfer fails and the user starts over.
    await panel.connect();
    await panel.chooseDestination();
    void panel.accept();
    pending[0].onEvent({ kind: 'failed', reason: 'connection lost' });
    await panel.connect();
    await panel.chooseDestination();
    void panel.accept();

    pending[0].resolve(1); // late response of the first accept
    pending[0].onEvent({ kind: 'completed', files: ['stale'] });
    await new Promise((resolve) => setTimeout(resolve)); // let the stale accept resume
    await panel.cancel();
    expect(p2p.cancelTransfer).not.toHaveBeenCalled(); // the current transfer has no id yet

    pending[1].resolve(2);
    await new Promise((resolve) => setTimeout(resolve)); // let the current accept resume
    await fixture.whenStable();
    await panel.cancel();

    expect(p2p.cancelTransfer).toHaveBeenCalledWith(2);
    expect(el.textContent).toContain('Receiving');
    expect(el.textContent).not.toContain('stale');
  });

  it('still resets after a reject when a destination was picked meanwhile', async () => {
    let finishReject: () => void = () => {};
    const { panel, fixture, el } = setup({
      rejectOffer: vi.fn(() => new Promise<void>((resolve) => (finishReject = resolve))),
    });
    await panel.connect();
    const rejecting = panel.reject();
    await panel.chooseDestination();
    finishReject();
    await rejecting;
    await fixture.whenStable();
    expect(el.textContent).toContain('Paste the ticket');
  });
});
