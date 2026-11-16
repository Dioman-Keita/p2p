import { TestBed } from '@angular/core/testing';

import { App } from './app';
import { P2pService } from './core/p2p.service';

describe('App', () => {
  it('shows the send panel first and switches to receive', async () => {
    TestBed.configureTestingModule({ providers: [{ provide: P2pService, useValue: {} }] });
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    const send = el.querySelector('app-send-panel') as HTMLElement;
    const receive = el.querySelector('app-receive-panel') as HTMLElement;
    expect(send.hidden).toBe(false);
    expect(receive.hidden).toBe(true);

    (el.querySelectorAll('nav button')[1] as HTMLButtonElement).click();
    await fixture.whenStable();
    expect(send.hidden).toBe(true);
    expect(receive.hidden).toBe(false);
  });
});
