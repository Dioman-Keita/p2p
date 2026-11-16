import { TestBed } from '@angular/core/testing';

import { P2pService } from '../core/p2p.service';
import { SendPanel } from './send-panel';

describe('SendPanel', () => {
  function setup(overrides: Partial<P2pService> = {}) {
    const p2p = {
      pickFiles: vi.fn(async () => ['C:\\a.txt', 'C:\\b.txt']),
      pickFolders: vi.fn(async () => ['C:\\photos']),
      share: vi.fn(async () => ({
        ticket: 'blobTICKET',
        files: [{ name: 'a.txt', size: 3 }],
        totalBytes: 3,
      })),
      unshare: vi.fn(async () => {}),
      ...overrides,
    };
    TestBed.configureTestingModule({ providers: [{ provide: P2pService, useValue: p2p }] });
    const fixture = TestBed.createComponent(SendPanel);
    return {
      fixture,
      panel: fixture.componentInstance,
      p2p,
      el: fixture.nativeElement as HTMLElement,
    };
  }

  it('collects files and folders without duplicates', async () => {
    const { panel, fixture, el } = setup();
    await panel.addFiles();
    await panel.addFiles();
    await panel.addFolders();
    await fixture.whenStable();
    expect(el.querySelectorAll('.files li').length).toBe(3);
  });

  it('shares the selection and shows the ticket', async () => {
    const { panel, fixture, el, p2p } = setup();
    await panel.addFiles();
    await panel.startSharing();
    await fixture.whenStable();
    expect(p2p.share).toHaveBeenCalledWith(['C:\\a.txt', 'C:\\b.txt']);
    expect((el.querySelector('textarea') as HTMLTextAreaElement).value).toBe('blobTICKET');
  });

  it('stops sharing', async () => {
    const { panel, fixture, el, p2p } = setup();
    await panel.addFiles();
    await panel.startSharing();
    await panel.stopSharing();
    await fixture.whenStable();
    expect(p2p.unshare).toHaveBeenCalledWith('blobTICKET');
    expect(el.textContent).toContain('No files selected yet.');
  });

  it('shows engine errors', async () => {
    const { panel, fixture, el } = setup({
      share: vi.fn(async () => Promise.reject('nothing to share')),
    });
    await panel.addFolders();
    await panel.startSharing();
    await fixture.whenStable();
    expect(el.querySelector('[role=alert]')?.textContent).toContain('nothing to share');
  });
});
