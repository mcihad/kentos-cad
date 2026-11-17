// Signing in and out of the portal: who the person is afterwards, and where they go.
import { portal } from '../api/portal';
import { go } from './router';
import { contexts, homeOf, me, unread } from './state';

/** After signing in or up: the account and its places, then the page asked for or the first place. */
export async function signedIn(next?: string | null): Promise<void> {
  const m = await portal.me();
  me.set(m);
  unread.set(m.unread);
  const first = contexts(m)[0];
  go(next && next.startsWith('#/') ? next : first ? homeOf(first) : '#/');
}
