/**
 * An invitation's link, `…/?davet=<token>` (docs/adr/0035, 0042). The token
 * is a one-time secret: it is taken off the address as the page starts,
 * before anything else is fetched (main.ts), and kept only for this tab —
 * in memory and in sessionStorage, never in localStorage — so it survives a
 * sign-in, also one that leaves the page (OpenID), until the invitation is
 * accepted or its window is closed. It is never logged. This module is
 * loaded with the page, so it imports nothing.
 */

/** The address parameter of an invitation's link. */
export const INVITE_PARAM = 'davet';

/** Where this tab keeps a link's token until it is used (sessionStorage). */
export const INVITE_KEY = 'kentos.invitation';

/** What the link needs of the browser (tests pass their own). */
export interface LinkHost {
  location: Pick<Location, 'search' | 'pathname' | 'hash'>;
  history: Pick<History, 'replaceState' | 'state'>;
  /** This tab's storage; null when the browser refuses it (the token then lives in memory only). */
  session: Pick<Storage, 'getItem' | 'setItem' | 'removeItem'> | null;
}

function browserHost(): LinkHost {
  let session: LinkHost['session'] = null;
  try {
    session = window.sessionStorage;
  } catch {
    // Storage refused (privacy settings): memory only.
  }
  return { location: window.location, history: window.history, session };
}

/** The token while this page lives (also when sessionStorage is refused). */
let held: string | null = null;

/**
 * Takes an invitation's token off the address, if it carries one, and keeps
 * it for this tab. The other parameters and the hash stay as they were.
 */
export function takeInvitationLink(host: LinkHost = browserHost()): void {
  const q = new URLSearchParams(host.location.search);
  const token = q.get(INVITE_PARAM);
  if (token === null) return;
  q.delete(INVITE_PARAM);
  host.history.replaceState(host.history.state, '', `${host.location.pathname}${q.size ? `?${q}` : ''}${host.location.hash}`);
  const t = token.trim();
  if (!t) return;
  held = t;
  try {
    host.session?.setItem(INVITE_KEY, t);
  } catch {
    // Full or refused: memory only (a sign-in that leaves the page then loses it; the link still works).
  }
}

/** The token of a link this tab was opened with and has not used yet, or null. */
export function pendingInvitation(host: LinkHost = browserHost()): string | null {
  if (held) return held;
  try {
    return host.session?.getItem(INVITE_KEY) || null;
  } catch {
    return null;
  }
}

/** Forgets the token: the invitation was accepted, refused for good, or its window closed. */
export function dropInvitation(host: LinkHost = browserHost()): void {
  held = null;
  try {
    host.session?.removeItem(INVITE_KEY);
  } catch {
    // Nothing kept there.
  }
}
