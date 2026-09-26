import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { GrantRole } from '../../contracts/generated/GrantRole';
import type { InvitationAccepted } from '../../contracts/generated/InvitationAccepted';
import type { InvitationState } from '../../contracts/generated/InvitationState';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectInvitation } from '../../contracts/generated/ProjectInvitation';
import type { ProjectInvite } from '../../contracts/generated/ProjectInvite';
import { INVITE_PARAM } from './invitationLink';
import { ROLE_LABEL, dateText } from './sharing';

/**
 * Inviting someone to a cloud project by e-mail (docs/adr/0035, 0042;
 * TODOS.md CLOUD-16, CLOUD-17): the rules the server keeps, as the share
 * dialog shows them before it asks (the roles an invitation gives, how long
 * it waits, an e-mail's form), the product commands `project.invite` and
 * `project.invitation.revoke`, the link the inviter sends, and the words
 * for an invitation's state and for what an accepted one gave. The server
 * decides every request; these only keep a form from promising what it
 * would refuse.
 */

/** The roles an invitation gives, weakest first: managing and sharing stay with members. */
export const INVITE_ROLES: readonly GrantRole[] = ['viewer', 'commenter', 'editor'];

/** How long an invitation waits unless told otherwise, and at most (days). */
export const INVITE_DAYS = 14;
export const INVITE_MAX_DAYS = 90;

/** The waits the form offers, in days. */
export const INVITE_DAY_CHOICES: readonly number[] = [1, 3, 7, 14, 30, 60, 90];

const DAY_MS = 86_400_000;
/** The longest wait is sent this much inside the limit: a clock a little ahead of the server's is not refused. */
const LIMIT_MARGIN_MS = 10 * 60_000;

export const INVITATION_STATE_LABEL: Record<InvitationState, string> = {
  pending: 'Bekliyor',
  accepted: 'Kabul edildi',
  revoked: 'Geri alındı',
  expired: 'Süresi doldu',
};

/**
 * Why `email` is not an address the server takes (its `check_email`:
 * trimmed, one `@` with something before it and a dotted domain after it,
 * no spaces, at most 254 characters), or null.
 */
export function emailProblem(email: string): string | null {
  const e = email.trim().toLowerCase();
  if (!e) return 'Davet edilecek kişinin e-posta adresini yazın.';
  const at = e.indexOf('@');
  const domain = e.slice(at + 1);
  const ok =
    e.length <= 254 &&
    !/[\s\p{Cc}]/u.test(e) &&
    at > 0 &&
    !domain.includes('@') &&
    domain.includes('.') &&
    !domain.startsWith('.') &&
    !domain.endsWith('.');
  return ok ? null : 'Davet için geçerli bir e-posta adresi yazın (ör. ad@kurum.gov.tr).';
}

/**
 * The `expiresAt` of an invitation that waits `days` days: none for the
 * default (the server's own 14 days), and the longest a little inside the
 * limit.
 */
export function inviteExpiry(days: number, now = Date.now()): string | undefined {
  if (days === INVITE_DAYS) return undefined;
  const wait = Math.min(days * DAY_MS, INVITE_MAX_DAYS * DAY_MS - LIMIT_MARGIN_MS);
  return new Date(now + wait).toISOString();
}

const envelope = (commandName: string, tenantId: string, projectId: string, input: unknown): CommandEnvelope => ({
  commandName,
  version: 1,
  tenantId,
  projectId,
  requestId: `web-${crypto.randomUUID()}`,
  idempotencyKey: crypto.randomUUID(),
  expectedVersions: {},
  input: input as CommandEnvelope['input'],
});

/** `project.invite` v1: an invitation for `email` with `role`, waiting until `expiresAt` (the server's 14 days when absent). */
export function inviteEnvelope(tenantId: string, projectId: string, email: string, role: GrantRole, expiresAt?: string): CommandEnvelope {
  const input: ProjectInvite = { email: email.trim(), role };
  if (expiresAt) input.expiresAt = expiresAt;
  return envelope('project.invite', tenantId, projectId, input);
}

/** `project.invitation.revoke` v1: a waiting invitation withdrawn (its link stops working). */
export function invitationRevokeEnvelope(tenantId: string, projectId: string, invitationId: string): CommandEnvelope {
  return envelope('project.invitation.revoke', tenantId, projectId, { invitationId });
}

/** The link the inviter sends: this app's address with the token. It is shown once and kept nowhere. */
export function invitationLink(token: string, base = `${location.origin}${location.pathname}`): string {
  return `${base}?${new URLSearchParams({ [INVITE_PARAM]: token })}`;
}

/** The invitations as the dialog lists them: the waiting ones first, each part newest first. */
export function sortInvitations(list: readonly ProjectInvitation[]): ProjectInvitation[] {
  const at = (i: ProjectInvitation) => Date.parse(i.createdAt);
  return [...list].sort((a, b) => Number(b.state === 'pending') - Number(a.state === 'pending') || at(b) - at(a));
}

/** One line under an invitation's e-mail: who invited when, and how it ended or when it stops waiting. */
export function invitationSub(i: ProjectInvitation): string {
  const by = `Davet eden: ${i.createdByName || 'görünmüyor'}, ${dateText(i.createdAt)}`;
  switch (i.state) {
    case 'pending':
      return `${by} · ${dateText(i.expiresAt)} tarihine kadar bekler`;
    case 'accepted':
      return `${by} · Kabul eden: ${i.acceptedByName || 'görünmüyor'}${i.acceptedAt ? `, ${dateText(i.acceptedAt)}` : ''}`;
    case 'expired':
      return `${by} · Süresi ${dateText(i.expiresAt)} tarihinde doldu`;
    default:
      return by;
  }
}

/**
 * What to ask before inviting `email`, or null: a waiting invitation for
 * the same address is replaced (its link stops working), and someone who
 * can use the project already keeps a stronger role.
 */
export function inviteQuestion(
  email: string,
  invitations: readonly ProjectInvitation[],
  access: ProjectAccessList | null,
): { waiting?: ProjectInvitation; holder?: { name: string; role: string } } | null {
  const e = email.trim().toLowerCase();
  const waiting = invitations.find((i) => i.state === 'pending' && i.email === e);
  const person = access?.people.find((p) => p.role && p.email?.toLowerCase() === e);
  if (!waiting && !person) return null;
  return {
    ...(waiting ? { waiting } : {}),
    ...(person?.role ? { holder: { name: person.displayName || e, role: ROLE_LABEL[person.role] } } : {}),
  };
}

/** How the account reaches a project an invitation opened, in one line. */
export function acceptedHow(a: InvitationAccepted): string {
  if (a.role === 'owner') return 'Projenin sahibisiniz; davet bir şey değiştirmedi.';
  if (a.guest) return 'Misafir olarak: kurumun dışındasınız ve yalnız bu projeyi görürsünüz.';
  if (a.tenantKind === 'personal') return 'Paylaşımla: proje bir kişisel alanda.';
  return 'Kurum üyesi olarak paylaşımla.';
}
