import { Signal } from '../../core/signal';
import type { InvitationAccepted } from '../../contracts/generated/InvitationAccepted';
import { ApiFailure, type CloudApi } from './api';
import type { AuthState } from './session';

/**
 * Accepting the invitation of a link this tab was opened with (docs/adr/0035,
 * 0042): as soon as someone is signed in, the token goes to the server once;
 * the answer is the project it opened, or the server's reason for refusing,
 * in its own words (another e-mail's invitation, an unverified e-mail, the
 * organisation takes no guests, a membership or seat not active, the project
 * deleted, the link used, withdrawn or out of date). A refusal another
 * account could get past (403) keeps the token, so signing in with the
 * invited address's account accepts it; any other answer ends it.
 */

export type AcceptState =
  /** Who is signed in is not known yet (the server is being asked, or away). */
  | { kind: 'waiting' }
  /** No one is signed in: the invited address's account signs in first. */
  | { kind: 'signin' }
  | { kind: 'accepting' }
  | { kind: 'accepted'; result: InvitationAccepted }
  /** The server refused; `another`: signing in with another account may get past it (403). */
  | { kind: 'refused'; failure: ApiFailure; another: boolean }
  /** No answer came (the network, the server away): the same request may go again. */
  | { kind: 'failed'; message: string };

export interface AcceptanceOptions {
  auth: Signal<AuthState>;
  api: Pick<CloudApi, 'acceptInvitation'>;
  token: string;
  /** The token is used up or refused for good: forget it. */
  drop(): void;
}

export class InvitationAcceptance {
  readonly state = new Signal<AcceptState>({ kind: 'waiting' });
  private readonly o: AcceptanceOptions;
  private running = false;
  private stop: (() => void) | null = null;

  constructor(o: AcceptanceOptions) {
    this.o = o;
  }

  /** Follows who is signed in: accepts when someone is (again, after another account signs in). */
  start(): void {
    this.stop ??= this.o.auth.subscribe((a) => this.follow(a), true);
  }

  dispose(): void {
    this.stop?.();
    this.stop = null;
  }

  /** Tries again after no answer came. */
  retry(): void {
    if (this.state.value.kind === 'failed') void this.accept();
  }

  private follow(auth: AuthState): void {
    const now = this.state.value.kind;
    // An answer the server gave stays until another account is signed in (a 403 kept the token).
    if (now === 'accepted' || now === 'accepting') return;
    if (auth === 'signedIn') {
      if (now !== 'refused') void this.accept();
    } else this.state.set(auth === 'signedOut' ? { kind: 'signin' } : { kind: 'waiting' });
  }

  private async accept(): Promise<void> {
    if (this.running) return;
    this.running = true;
    this.state.set({ kind: 'accepting' });
    try {
      const result = await this.o.api.acceptInvitation(this.o.token);
      this.o.drop();
      this.state.set({ kind: 'accepted', result });
    } catch (e) {
      const f = e instanceof ApiFailure ? e : new ApiFailure(0, { error: 'network' }, e instanceof Error ? e.message : String(e));
      if (f.status === 401) this.state.set({ kind: 'signin' });
      else if (f.transient) this.state.set({ kind: 'failed', message: f.code === 'network' ? 'Sunucuya ulaşılamadı.' : f.message });
      else {
        const another = f.status === 403;
        if (!another) this.o.drop();
        this.state.set({ kind: 'refused', failure: f, another });
      }
    } finally {
      this.running = false;
    }
  }
}
