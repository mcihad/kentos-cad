import { afterEach, describe, expect, it } from 'vitest';
import { Signal } from '../../core/signal';
import type { InvitationChange } from '../../contracts/generated/InvitationChange';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import { InvitationAcceptance, type AcceptState } from './acceptance';
import { FakeServer } from './fakeServer';
import { INVITE_KEY, dropInvitation, pendingInvitation, takeInvitationLink, type LinkHost } from './invitationLink';
import {
  INVITE_ROLES,
  acceptedHow,
  emailProblem,
  invitationLink,
  invitationRevokeEnvelope,
  invitationSub,
  inviteEnvelope,
  inviteExpiry,
  inviteQuestion,
  sortInvitations,
} from './invitations';
import type { AuthState } from './session';

/**
 * Invitations in the web (docs/adr/0035, 0042; TODOS.md CLOUD-16, CLOUD-17):
 * the link's token taken off the address and kept only for this tab; the
 * rules a form shows before the server decides; inviting, the one waiting
 * invitation per address, withdrawing; and accepting a link as the account
 * signed in: each outcome in the server's words, the token kept only where
 * another account could get past the refusal.
 */

function host(search: string) {
  const session = new Map<string, string>();
  const replaced: string[] = [];
  const h: LinkHost = {
    location: { search, pathname: '/cad/', hash: '#yer' },
    history: { state: { keep: 1 }, replaceState: (_s: unknown, _t: string, url?: string | URL | null) => void replaced.push(String(url)) },
    session: { getItem: (k) => session.get(k) ?? null, setItem: (k, v) => void session.set(k, v), removeItem: (k) => void session.delete(k) },
  };
  return { h, session, replaced };
}

const TOKEN = 'a'.repeat(64);

afterEach(() => dropInvitation(host('').h));

describe('the link of an invitation (?davet=…)', () => {
  it('is taken off the address at once and kept for this tab only, the other parameters and the hash left', () => {
    const { h, session, replaced } = host(`?renderer=webgl2&davet=${TOKEN}&start=0`);
    takeInvitationLink(h);
    expect(replaced).toEqual(['/cad/?renderer=webgl2&start=0#yer']);
    expect([session.get(INVITE_KEY), pendingInvitation(h)]).toEqual([TOKEN, TOKEN]);
    dropInvitation(h);
    expect([session.size, pendingInvitation(h)]).toEqual([0, null]);
  });

  it('survives a sign-in that leaves the page (sessionStorage), and a browser that refuses the storage (memory)', () => {
    const first = host(`?davet=${TOKEN}`);
    takeInvitationLink(first.h);
    // Back from the identity provider: a new page, the same tab's storage.
    const back = host('');
    back.session.set(INVITE_KEY, TOKEN);
    dropInvitation({ ...back.h, session: null });
    expect(pendingInvitation(back.h)).toBe(TOKEN);
    const refused = host(`?davet=${TOKEN}`);
    takeInvitationLink({ ...refused.h, session: null });
    expect([refused.replaced, pendingInvitation({ ...refused.h, session: null })]).toEqual([['/cad/#yer'], TOKEN]);
  });

  it('leaves an address without one alone; an empty one is taken off and kept nowhere', () => {
    const none = host('?start=0');
    takeInvitationLink(none.h);
    expect([none.replaced, pendingInvitation(none.h)]).toEqual([[], null]);
    const empty = host('?davet=%20');
    takeInvitationLink(empty.h);
    expect([empty.replaced, empty.session.size]).toEqual([['/cad/#yer'], 0]);
  });

  it('is the app’s own address with the token; nothing else', () => {
    expect(invitationLink(TOKEN, 'https://kentos.example/cad/')).toBe(`https://kentos.example/cad/?davet=${TOKEN}`);
  });
});

describe('what an invitation form checks before the server decides (docs/adr/0035)', () => {
  it('takes the e-mails the server takes, and no manager', () => {
    expect(emailProblem('  Ayse@Kurum.Gov.TR ')).toBeNull();
    for (const bad of ['', 'ayse', '@kurum.gov.tr', 'ayse@', 'ayse@kurum', 'a b@kurum.tr', 'a@b@c.tr', 'ayse@.tr', 'ayse@kurum.']) expect(emailProblem(bad), bad).not.toBeNull();
    expect(INVITE_ROLES).toEqual(['viewer', 'commenter', 'editor']);
  });

  it('waits 14 days unless told otherwise, and the longest inside 90', () => {
    const now = Date.parse('2026-09-26T10:00:00Z');
    expect(inviteExpiry(14, now)).toBeUndefined();
    expect(inviteExpiry(7, now)).toBe('2026-10-03T10:00:00.000Z');
    const longest = Date.parse(inviteExpiry(90, now)!);
    expect([longest < now + 90 * 86_400_000, longest > now + 89.99 * 86_400_000]).toEqual([true, true]);
    const e = inviteEnvelope('t', 'p', ' ad@kurum.tr ', 'editor', '2026-10-03T10:00:00.000Z');
    expect([e.commandName, e.version, e.input]).toEqual(['project.invite', 1, { email: 'ad@kurum.tr', role: 'editor', expiresAt: '2026-10-03T10:00:00.000Z' }]);
    expect(inviteEnvelope('t', 'p', 'ad@kurum.tr', 'viewer').input).toEqual({ email: 'ad@kurum.tr', role: 'viewer' });
  });

  it('asks before replacing a waiting invitation, or inviting someone who can use the project', () => {
    const waiting = { id: 'i1', email: 'ad@kurum.tr', role: 'viewer' as const, state: 'pending' as const, createdBy: 'u1', createdByName: 'Ayşe', createdAt: '2026-09-26T10:00:00Z', expiresAt: '2026-10-10T10:00:00Z' };
    const access = { people: [{ userId: 'u2', displayName: 'Mehmet Demir', email: 'Mehmet@Kurum.tr', role: 'editor', expired: false, guest: false }] } as unknown as ProjectAccessList;
    expect(inviteQuestion('AD@kurum.tr', [waiting], access)).toEqual({ waiting });
    expect(inviteQuestion('mehmet@kurum.tr', [], access)).toEqual({ holder: { name: 'Mehmet Demir', role: 'Düzenleyici' } });
    expect(inviteQuestion('yeni@kurum.tr', [{ ...waiting, state: 'revoked' }], access)).toBeNull();
  });

  it('lists the waiting ones first, and says each one’s story', () => {
    const at = (id: string, state: 'pending' | 'accepted' | 'revoked' | 'expired', createdAt: string) => ({
      id,
      email: `${id}@kurum.tr`,
      role: 'viewer' as const,
      state,
      createdBy: 'u1',
      createdByName: 'Ayşe Yılmaz',
      createdAt,
      expiresAt: '2026-10-10T10:00:00Z',
      ...(state === 'accepted' ? { acceptedByName: 'Misafir Kişi', acceptedAt: '2026-09-27T09:00:00Z' } : {}),
    });
    const list = [at('eski', 'pending', '2026-09-20T10:00:00Z'), at('kabul', 'accepted', '2026-09-26T10:00:00Z'), at('yeni', 'pending', '2026-09-25T10:00:00Z')];
    expect(sortInvitations(list).map((i) => i.id)).toEqual(['yeni', 'eski', 'kabul']);
    expect(invitationSub(list[0])).toBe('Davet eden: Ayşe Yılmaz, 20.09.2026 · 10.10.2026 tarihine kadar bekler');
    expect(invitationSub(list[1])).toBe('Davet eden: Ayşe Yılmaz, 26.09.2026 · Kabul eden: Misafir Kişi, 27.09.2026');
  });
});

/** The fake server with the invitations of a project named “Ada 101”. */
function server() {
  return new FakeServer({ name: 'Ada 101', settings: {} as never, layers: [], activeLayer: 'x', styles: { items: [], categories: [] }, origin: { x: 0, y: 0 } });
}

describe('inviting and withdrawing (docs/adr/0035)', () => {
  it('gives the link’s token in the first answer only; the list and a retry never have it', async () => {
    const s = server();
    const e = inviteEnvelope('t', 'p', 'Misafir@Example.org', 'editor');
    const first = await s.lifecycle<InvitationChange>(e);
    expect([first.token?.length, first.invitation.email, first.invitation.state]).toEqual([64, 'misafir@example.org', 'pending']);
    const again = await s.lifecycle<InvitationChange>(e);
    expect([again.token, again.replayed]).toEqual([undefined, true]);
    expect(JSON.stringify(await s.invitations())).not.toContain(first.token!);
  });

  it('a new invitation to the same address replaces the waiting one: its link stops working', async () => {
    const s = server();
    const old = await s.lifecycle<InvitationChange>(inviteEnvelope('t', 'p', 'misafir@example.org', 'viewer'));
    await s.lifecycle<InvitationChange>(inviteEnvelope('t', 'p', 'misafir@example.org', 'editor'));
    expect((await s.invitations()).invitations.map((i) => i.state)).toEqual(['pending', 'revoked']);
    await expect(s.acceptInvitation(old.token!)).rejects.toMatchObject({ status: 404 });
  });

  it('withdrawn, a link does not work; only those who may share invite, and not as managers', async () => {
    const s = server();
    const made = await s.lifecycle<InvitationChange>(inviteEnvelope('t', 'p', 'misafir@example.org', 'editor'));
    const gone = await s.lifecycle<InvitationChange>(invitationRevokeEnvelope('t', 'p', made.invitation.id));
    expect(gone.invitation.state).toBe('revoked');
    await expect(s.acceptInvitation(made.token!)).rejects.toMatchObject({ status: 404 });
    await expect(s.lifecycle(invitationRevokeEnvelope('t', 'p', made.invitation.id))).rejects.toMatchObject({ status: 404 });
    await expect(s.lifecycle(inviteEnvelope('t', 'p', 'misafir@example.org', 'manager'))).rejects.toMatchObject({ path: 'role' });
    s.role = 'editor';
    await expect(s.lifecycle(inviteEnvelope('t', 'p', 'misafir@example.org', 'viewer'))).rejects.toMatchObject({ code: 'forbidden' });
    await expect(s.invitations()).rejects.toMatchObject({ code: 'forbidden' });
  });
});

describe('accepting a link (docs/adr/0035, 0042)', () => {
  async function flow(opts: { auth?: AuthState } = {}) {
    const s = server();
    const made = await s.lifecycle<InvitationChange>(inviteEnvelope('t', 'p', 'misafir@example.org', 'editor'));
    const auth = new Signal<AuthState>(opts.auth ?? 'signedIn');
    let dropped = 0;
    const a = new InvitationAcceptance({ auth, api: s, token: made.token!, drop: () => dropped++ });
    const seen: AcceptState['kind'][] = [];
    a.state.subscribe((st) => seen.push(st.kind));
    a.start();
    const settle = async () => {
      for (let i = 0; i < 20 && ['accepting', 'waiting'].includes(a.state.value.kind) && auth.value === 'signedIn'; i++) await new Promise((r) => setTimeout(r, 1));
    };
    await settle();
    return { s, a, auth, seen, settle, dropped: () => dropped };
  }

  it('signed out, it waits for the sign-in, then accepts once: a guest outside the organisation, an editor', async () => {
    const f = await flow({ auth: 'signedOut' });
    expect(f.a.state.value.kind).toBe('signin');
    f.auth.set('signedIn');
    await f.settle();
    const st = f.a.state.value;
    expect(st.kind === 'accepted' && [st.result.projectName, st.result.role, st.result.guest, acceptedHow(st.result)]).toEqual([
      'Ada 101',
      'editor',
      true,
      'Misafir olarak: kurumun dışındasınız ve yalnız bu projeyi görürsünüz.',
    ]);
    expect([f.dropped(), f.seen, f.s.invites.granted.get('u9')]).toEqual([1, ['signin', 'accepting', 'accepted'], { role: 'editor', guest: true }]);
    // Signing in again later does not send it a second time.
    f.auth.set('signedOut');
    f.auth.set('signedIn');
    expect(f.a.state.value.kind).toBe('accepted');
  });

  it('another e-mail’s account: refused in the server’s words, the token kept; the invited account then accepts it', async () => {
    const f = await flow({ auth: 'signedOut' });
    f.s.invites.taker = { id: 'u2', name: 'Mehmet Demir', verified: true, member: 'active' };
    f.auth.set('signedIn');
    await f.settle();
    const st = f.a.state.value;
    expect(st.kind === 'refused' && [st.another, st.failure.status, st.failure.message]).toEqual([true, 403, 'Bu davet başka bir e-posta adresi için. Davetin gönderildiği adresin hesabıyla giriş yapın.']);
    expect(f.dropped()).toBe(0);
    // Signed out, and in as the invited address's account.
    f.s.invites.taker = { id: 'u9', name: 'Misafir Kişi', email: 'MISAFIR@example.org', verified: true };
    f.auth.set('signedOut');
    expect(f.a.state.value.kind).toBe('signin');
    f.auth.set('signedIn');
    await f.settle();
    expect([f.a.state.value.kind, f.dropped()]).toEqual(['accepted', 1]);
  });

  it('each refusal says its own reason; only a 403 keeps the token', async () => {
    const cases: [string, (f: Awaited<ReturnType<typeof flow>>) => void, number, RegExp, boolean][] = [
      ['unverified', (f) => (f.s.invites.taker!.verified = false), 403, /e-posta adresi doğrulanmamış/, true],
      ['guests_off', (f) => (f.s.invites.guestsAllowed = false), 403, /misafir kabul etmiyor/, true],
      ['inactive', (f) => (f.s.invites.taker!.member = 'inactive'), 403, /üyeliğiniz ya da koltuğunuz etkin değil/, true],
      ['gone', (f) => (f.s.deleted = true), 410, /proje silinmiş ya da kurumu etkin değil/, false],
      ['missing', (f) => (f.s.invites.invitations[0].state = 'revoked'), 404, /süresi dolmuş, kullanılmış ya da geri alınmış/, false],
    ];
    for (const [name, arrange, status, words, kept] of cases) {
      const f = await flow({ auth: 'signedOut' });
      arrange(f);
      f.auth.set('signedIn');
      await f.settle();
      const st = f.a.state.value;
      expect(st.kind === 'refused' && [st.failure.status, words.test(st.failure.message), st.another, f.dropped() === 0], name).toEqual([status, true, kept, kept]);
    }
  });

  it('the project’s owner stays its owner; a member keeps a stronger role and is no guest', async () => {
    const owner = await flow({ auth: 'signedOut' });
    owner.s.invites.taker!.owner = true;
    owner.auth.set('signedIn');
    await owner.settle();
    const o = owner.a.state.value;
    expect(o.kind === 'accepted' && [o.result.role, acceptedHow(o.result)]).toEqual(['owner', 'Projenin sahibisiniz; davet bir şey değiştirmedi.']);
    const member = await flow({ auth: 'signedOut' });
    Object.assign(member.s.invites.taker!, { member: 'active', holds: 'manager' });
    member.auth.set('signedIn');
    await member.settle();
    const m = member.a.state.value;
    expect(m.kind === 'accepted' && [m.result.role, m.result.guest, acceptedHow(m.result)]).toEqual(['manager', false, 'Kurum üyesi olarak paylaşımla.']);
  });

  it('no answer: said, and tried again when asked; a session that ended asks to sign in', async () => {
    const f = await flow({ auth: 'signedOut' });
    f.s.offline = true;
    f.auth.set('signedIn');
    await f.settle();
    expect(f.a.state.value).toEqual({ kind: 'failed', message: 'Sunucuya ulaşılamadı.' });
    f.s.offline = false;
    f.a.retry();
    await f.settle();
    expect(f.a.state.value.kind).toBe('accepted');
    const ended = await flow({ auth: 'signedOut' });
    ended.s.invites.taker = null;
    ended.auth.set('signedIn');
    await ended.settle();
    expect([ended.a.state.value.kind, ended.dropped()]).toEqual(['signin', 0]);
  });
});
