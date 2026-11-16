import { Component, computed, inject, signal } from '@angular/core';

import type { Share } from '../core/bindings/Share';
import { errorMessage, P2pService } from '../core/p2p.service';
import { FileSizePipe } from '../shared/file-size.pipe';

@Component({
  selector: 'app-send-panel',
  imports: [FileSizePipe],
  templateUrl: './send-panel.html',
})
export class SendPanel {
  private readonly p2p = inject(P2pService);

  protected readonly selection = signal<string[]>([]);
  protected readonly share = signal<Share | null>(null);
  protected readonly busy = signal(false);
  protected readonly error = signal<string | null>(null);
  protected readonly copied = signal(false);
  protected readonly canShare = computed(() => this.selection().length > 0 && !this.busy());

  async addFiles(): Promise<void> {
    this.add(await this.p2p.pickFiles());
  }

  async addFolders(): Promise<void> {
    this.add(await this.p2p.pickFolders());
  }

  remove(path: string): void {
    this.selection.update((paths) => paths.filter((p) => p !== path));
  }

  async startSharing(): Promise<void> {
    this.busy.set(true);
    this.error.set(null);
    try {
      this.share.set(await this.p2p.share(this.selection()));
    } catch (err) {
      this.error.set(errorMessage(err));
    } finally {
      this.busy.set(false);
    }
  }

  async copyTicket(): Promise<void> {
    const share = this.share();
    if (!share) return;
    await navigator.clipboard.writeText(share.ticket);
    this.copied.set(true);
    setTimeout(() => this.copied.set(false), 2000);
  }

  async stopSharing(): Promise<void> {
    const share = this.share();
    if (!share) return;
    await this.p2p.unshare(share.ticket);
    this.share.set(null);
    this.selection.set([]);
  }

  private add(paths: string[]): void {
    this.selection.update((current) => [...new Set([...current, ...paths])]);
  }
}
