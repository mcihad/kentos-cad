import { describe, expect, it } from 'vitest';
import type { AccessBlock } from '../../contracts/generated/AccessBlock';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { GrantRole } from '../../contracts/generated/GrantRole';
import type { InvitationAccepted } from '../../contracts/generated/InvitationAccepted';
import type { InvitationChange } from '../../contracts/generated/InvitationChange';
import type { InvitationState } from '../../contracts/generated/InvitationState';
import type { ProjectAccessHolder } from '../../contracts/generated/ProjectAccessHolder';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectInvitation } from '../../contracts/generated/ProjectInvitation';
import type { ProjectRole } from '../../contracts/generated/ProjectRole';
import { ApiFailure } from './api';
import {
  INVITATION_STATE_LABEL,
  INVITE_DAY_CHOICES,
  INVITE_DAYS,
  INVITE_MAX_DAYS,
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
import {
  INVITE_LINES,
  SHARE_TEXTS,
  candidateNote,
  dayText,
  invitationInitial,
  invitationRevokeQuestion,
  invitedLink,
  inviteAsk,
  inviteRules,
  inviteTip,
  invitesCount,
  nobodyFound,
  peopleView,
  revokeQuestion,
  revokedText,
  roleChangedText,
  searchesFor,
  shareLog,
  shareTip,
  sharedText,
} from './sharePlan';
import { BLOCK_TEXT, GRANT_ROLES, ROLE_HINT, ROLE_LABEL, STORAGE_TEXT, dateText, endOfDay, failureText, revokeEnvelope, shareEnvelope, sourceText } from './sharing';

/**
 * The share dialog's words and rules (fixtures/cloud/v1/share.json, format
 * in fixtures/cloud/README.md): the roles and what they allow, who may be
 * changed or taken away, the rows, the notes, the questions, the
 * invitations' checks, waits and links, the commands the dialog sends and
 * the lines it writes. The file's answers are worked out apart from this
 * code; the desktop's Paylaş window checks itself against the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/cloud/v1/share.json', { query: '?raw', import: 'default', eager: true });
type Holder = ProjectAccessHolder;
type Failure = { status?: number; error?: string; message?: string; retryable?: boolean; fallback?: string; plain?: string };
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  timeZone: string;
  roles: { labels: Record<ProjectRole, string>; grant: GrantRole[]; invite: GrantRole[]; hints: Record<GrantRole, string> };
  blocks: Record<AccessBlock, string>;
  storage: typeof STORAGE_TEXT;
  invitationStates: Record<InvitationState, string>;
  inviteDays: { default: number; max: number; choices: { days: number; text: string }[] };
  texts: unknown;
  access: ProjectAccessList;
  people: { id: string; me: string; mayShare: boolean; list: Partial<ProjectAccessList>; expect: unknown }[];
  sources: { holder: Holder; text: string }[];
  shareTips: { mayShare: boolean; chosen: boolean; text: string }[];
  shared: { name: string; role: GrantRole; changed: boolean; had: boolean; text: string }[];
  revokeQuestions: { name: string; project: string; guest: boolean; expect: unknown }[];
  finder: {
    nobody: { query: string; personal: boolean; expect: unknown }[];
    notes: { role: ProjectRole | null; text: string | null }[];
    searches: { query: string; searches: boolean }[];
  };
  emails: { email: string; problem: string | null }[];
  expiry: { days: number; now: string; expiresAt: string | null }[];
  endOfDay: { date: string; expiresAt: string | null }[];
  dates: { iso: string; text: string }[];
  inviteTips: { mayShare: boolean; email: string; text: string }[];
  invitations: {
    sorted: { list: ProjectInvitation[]; order: string[] };
    counts: { states: InvitationState[]; text: string }[];
    subs: { invitation: ProjectInvitation; text: string }[];
    questions: { id: string; email: string; expect: { question: unknown; ask: unknown } | null }[];
    questionInvitations: ProjectInvitation[];
    rules: { personal: string; organization: string };
    links: { change: InvitationChange; days: number; expect: unknown }[];
    revokeQuestion: { email: string; expect: unknown };
    initials: { email: string; text: string }[];
    link: { token: string; base: string; link: string };
  };
  accepted: { accepted: InvitationAccepted; text: string }[];
  envelopes: { command: 'share' | 'revoke' | 'invite' | 'invitationRevoke'; args: string[]; expect: unknown }[];
  failures: { error: Failure; failed: string; text: string }[];
  lines: { line: string; args: string[]; text: string }[];
};

// Dates are the device's local time; the cases fix the zone (Node reads TZ again when it changes).
(globalThis as unknown as { process: { env: Record<string, string> } }).process.env.TZ = F.timeZone;

const plain = (v: unknown): unknown => JSON.parse(JSON.stringify(v));

/** The sample each text made from a value is written with, by its place in SHARE_TEXTS. */
const SAMPLES: Record<string, unknown> = {
  'people.removeLabel': 'Mehmet Demir',
  'people.roleLabel': 'Mehmet Demir',
  'people.count': 3,
  'people.adding': 'Mehmet Demir',
  'people.changing': 'Mehmet Demir',
  'people.revoking': 'Mehmet Demir',
  'people.storage': 'Yönetilen PostGIS veritabanı',
  'find.has': 'Düzenleyici',
  'invites.count': 2,
  'invites.revokeLabel': 'ali@ornek.org',
  'invites.inviting': 'ali@ornek.org',
  'invites.revoking': 'ali@ornek.org',
};

/** SHARE_TEXTS as the file writes them: a text made from a value as `{ sample, text }`. */
function fileTexts(o: object, path = ''): unknown {
  return Object.fromEntries(
    Object.entries(o).map(([k, v]) => {
      const at = path ? `${path}.${k}` : k;
      if (typeof v === 'function') {
        if (!(at in SAMPLES)) throw new Error(`Metin örneği yok: ${at}`);
        return [k, { sample: SAMPLES[at], text: (v as (x: unknown) => string)(SAMPLES[at]) }];
      }
      return [k, v && typeof v === 'object' ? fileTexts(v as object, at) : v];
    }),
  );
}

const invitation = (state: InvitationState, i: number): ProjectInvitation => ({ ...F.invitations.sorted.list[0], id: `x${i}`, state });

function failure(e: Failure): unknown {
  if (e.plain !== undefined) return new Error(e.plain);
  return new ApiFailure(e.status ?? 0, { error: e.error, message: e.message, retryable: e.retryable }, e.fallback ?? e.message ?? '');
}

const ENVELOPE: Record<string, (...a: string[]) => CommandEnvelope> = {
  share: (t, p, u, r, x) => shareEnvelope(t, p, u, r as GrantRole, x),
  revoke: (t, p, u) => revokeEnvelope(t, p, u),
  invite: (t, p, e, r, x) => inviteEnvelope(t, p, e, r as GrantRole, x),
  invitationRevoke: (t, p, i) => invitationRevokeEnvelope(t, p, i),
};

const LINES: Record<string, (...a: string[]) => string> = {
  shareLog: (p, t) => shareLog(p, t),
  roleChanged: (n, r) => roleChangedText(n, r as GrantRole),
  revoked: (n) => revokedText(n),
  invitedLog: (p, e, r) => INVITE_LINES.invitedLog(p, e, r as GrantRole),
  invitationRevokedLog: (p, e) => INVITE_LINES.revokedLog(p, e),
  invitationRevokedSay: (e) => INVITE_LINES.revokedSay(e),
};

describe('share dialog words and rules (fixtures/cloud/v1/share.json)', () => {
  it('is a v1 share file', () => {
    expect([F.format, F.version]).toEqual(['kentos.share', 1]);
  });

  it('names the roles, what each allows, which a share and an invitation give, the blocks, the storage and the invitations’ states', () => {
    expect([ROLE_LABEL, GRANT_ROLES, INVITE_ROLES, ROLE_HINT]).toEqual([F.roles.labels, F.roles.grant, F.roles.invite, F.roles.hints]);
    expect([BLOCK_TEXT, STORAGE_TEXT, INVITATION_STATE_LABEL]).toEqual([F.blocks, F.storage, F.invitationStates]);
    expect([INVITE_DAYS, INVITE_MAX_DAYS]).toEqual([F.inviteDays.default, F.inviteDays.max]);
    expect(INVITE_DAY_CHOICES.map((days) => ({ days, text: dayText(days) }))).toEqual(F.inviteDays.choices);
  });

  it('says the dialog’s words', () => {
    expect(fileTexts(SHARE_TEXTS)).toEqual(F.texts);
  });

  for (const c of F.people)
    it(`lists the people: ${c.id}`, () => {
      expect(plain(peopleView({ ...F.access, ...c.list }, c.me, c.mayShare))).toEqual(c.expect);
    });

  it('says where each access comes from, or why not', () => {
    for (const c of F.sources) expect(sourceText(c.holder), c.holder.userId).toBe(c.text);
  });

  it('says why Paylaş is off, what a share did, and asks before taking access away', () => {
    for (const c of F.shareTips) expect(shareTip(c.mayShare, c.chosen), JSON.stringify(c)).toBe(c.text);
    for (const c of F.shared) expect(sharedText(c.name, c.role, c.changed, c.had), JSON.stringify(c)).toBe(c.text);
    for (const c of F.revokeQuestions) expect(revokeQuestion(c.name, c.project, c.guest), c.name).toEqual(c.expect);
  });

  it('finds people: from two letters, the role of one who has one, and an invitation for a whole e-mail nobody has', () => {
    for (const c of F.finder.nobody) expect(nobodyFound(c.query, c.personal), JSON.stringify(c)).toEqual(c.expect);
    for (const c of F.finder.notes) expect(candidateNote(c.role ?? undefined), String(c.role)).toBe(c.text);
    for (const c of F.finder.searches) expect(searchesFor(c.query), JSON.stringify(c.query)).toBe(c.searches);
  });

  it('checks an invitation’s e-mail as the server does, and its wait', () => {
    for (const c of F.emails) expect(emailProblem(c.email), JSON.stringify(c.email)).toBe(c.problem);
    for (const c of F.expiry) expect(inviteExpiry(c.days, Date.parse(c.now)) ?? null, String(c.days)).toBe(c.expiresAt);
    for (const c of F.inviteTips) expect(inviteTip(c.mayShare, c.email), JSON.stringify(c)).toBe(c.text);
  });

  it('ends a share at the end of the picked local day, refuses a day the month does not have, and writes dates as the interface does', () => {
    for (const c of F.endOfDay) expect(endOfDay(c.date), c.date).toBe(c.expiresAt);
    for (const c of F.dates) expect(dateText(c.iso), c.iso).toBe(c.text);
  });

  it('lists the invitations and says each one’s story', () => {
    expect(sortInvitations(F.invitations.sorted.list).map((i) => i.id)).toEqual(F.invitations.sorted.order);
    for (const c of F.invitations.counts) expect(invitesCount(c.states.map(invitation)), c.states.join()).toBe(c.text);
    for (const c of F.invitations.subs) expect(invitationSub(c.invitation), JSON.stringify(c.invitation)).toBe(c.text);
    expect([inviteRules(true), inviteRules(false)]).toEqual([F.invitations.rules.personal, F.invitations.rules.organization]);
    for (const c of F.invitations.initials) expect(invitationInitial(c.email)).toBe(c.text);
  });

  it('asks before replacing a waiting invitation or inviting someone who can use the project, and before withdrawing one', () => {
    for (const c of F.invitations.questions) {
      const q = inviteQuestion(c.email, F.invitations.questionInvitations, F.access);
      expect(q && plain({ question: q, ask: inviteAsk(c.email.trim(), q) }), c.id).toEqual(c.expect);
    }
    expect(invitationRevokeQuestion(F.invitations.revokeQuestion.email)).toEqual(F.invitations.revokeQuestion.expect);
  });

  it('shows a new invitation’s link once, says how to get one when a retry has none, and makes the link from the token', () => {
    for (const c of F.invitations.links) expect(invitedLink(c.change, c.days), String(c.days)).toEqual(c.expect);
    const l = F.invitations.link;
    expect(invitationLink(l.token, l.base)).toBe(l.link);
  });

  it('says how an accepted invitation reaches the project', () => {
    for (const c of F.accepted) expect(acceptedHow(c.accepted), JSON.stringify(c.accepted)).toBe(c.text);
  });

  it('sends the product commands with fresh request ids', () => {
    for (const c of F.envelopes) {
      const { requestId, idempotencyKey, ...rest } = ENVELOPE[c.command](...c.args);
      expect(requestId, c.command).toMatch(/^web-[0-9a-f-]{36}$/);
      expect(idempotencyKey, c.command).toMatch(/^[0-9a-f-]{36}$/);
      expect(rest, c.command).toEqual(c.expect);
    }
  });

  it('says what failed, why, and what to do', () => {
    for (const c of F.failures) expect(failureText(failure(c.error), c.failed), JSON.stringify(c.error)).toBe(c.text);
  });

  it('writes the lines of each change', () => {
    for (const c of F.lines) expect(LINES[c.line](...c.args), c.line).toBe(c.text);
  });
});
