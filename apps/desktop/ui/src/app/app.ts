import { Component, signal } from '@angular/core';

import { ReceivePanel } from './receive/receive-panel';
import { SendPanel } from './send/send-panel';

type Tab = 'send' | 'receive';

@Component({
  selector: 'app-root',
  imports: [SendPanel, ReceivePanel],
  template: `
    <header>
      <h1>p2p</h1>
      <nav role="tablist">
        <button role="tab" [attr.aria-selected]="tab() === 'send'" (click)="tab.set('send')">
          Send
        </button>
        <button role="tab" [attr.aria-selected]="tab() === 'receive'" (click)="tab.set('receive')">
          Receive
        </button>
      </nav>
    </header>
    <main>
      <!-- Both panels stay alive so switching tabs never loses a share or a transfer. -->
      <app-send-panel [hidden]="tab() !== 'send'" />
      <app-receive-panel [hidden]="tab() !== 'receive'" />
    </main>
  `,
})
export class App {
  protected readonly tab = signal<Tab>('send');
}
