import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { GrantRole } from '../../contracts/generated/GrantRole';
import type { InvitationAccepted } from '../../contracts/generated/InvitationAccepted';
import type { InvitationChange } from '../../contracts/generated/InvitationChange';
import type { InvitationRevoke } from '../../contracts/generated/InvitationRevoke';
import type { ProjectInvitation } from '../../contracts/generated/ProjectInvitation';
import type { ProjectInvitations } from '../../contracts/generated/ProjectInvitations';
import type { ProjectInvite } from '../../contracts/generated/ProjectInvite';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { TenantKind } from '../../contracts/generated/TenantKind';
import { ApiFailure } from './api';

/**
 * The invitations of the fake server (never used by the app): `project.invite`
 * and `project.invitation.revoke`, the list, and accepting a link's token by
 * the account a test chooses, with every outcome of
 * `kentos.accept_invitation` (migration 0009) in its order: missing,
 * wrong_email, unverified, gone, owner, inactive, guests_off, then the grant
 * (a guest outside an organisation; a stronger held role kept). It follows
 * crates/server/application/src/invitations.rs; the real thing is tested
 * against PostgreSQL in Rust and end to end in the browser.
 */

/** What the invitations need of the fake server. */
export interface InviteHost {
  readonly projectName: string;
  /** The inviter: the fake's caller. */
  readonly userId: string;
  permissions(): ProjectPermission[];
  /** Offline, access taken away (404), the project deleted (410). */
  guard(): void;
  deleted(): boolean;
}

/** The account that opens a link, as a test sets it up. */
export interface Taker {
  id: string;
  name: string;
  email?: string;
  verified: boolean;
  /** Its membership of the project's organisation: none, or one whose status or seat is not active. */
  member?: 'active' | 'inactive';
  owner?: boolean;
  /** A grant it holds already. */
  holds?: GrantRole;
}

const RANK: readonly GrantRole[] = ['viewer', 'commenter', 'editor', 'manager'];
const DAY_MS = 86_400_000;

const invalid = (message: string, path: string) => new ApiFailure(422, { error: 'invalid', message, path }, message);
const missing = () => new ApiFailure(404, { error: 'not_found', message: 'Davet bulunamadı ya da artık beklemiyor; listeyi yenileyin.' }, 'Yok.');

export class FakeInvites {
  readonly invitations: (ProjectInvitation & { token: string })[] = [];
  /** Who opens a link; null: nobody is signed in (401). */
  taker: Taker | null = { id: 'u9', name: 'Misafir Kişi', email: 'misafir@example.org', verified: true };
  tenantKind: TenantKind = 'organization';
  guestsAllowed = true;
  /** The grants acceptances gave, by account. */
  readonly granted = new Map<string, { role: GrantRole; guest: boolean }>();
  /** The next invitation is made, but its answer never arrives. */
  loseNextAnswer = false;
  private readonly log = new Map<string, InvitationChange>();
  private readonly host: InviteHost;
  private clock = 0;

  constructor(host: InviteHost) {
    this.host = host;
  }

  private may(permission: ProjectPermission): void {
    if (!this.host.permissions().includes(permission))
      throw new ApiFailure(403, { error: 'forbidden', message: `Bu işlem için yetkiniz yok (${permission}).` }, 'Yetki yok.');
  }

  /** `project.invite` and `project.invitation.revoke`; null for any other command. */
  command(envelope: CommandEnvelope): InvitationChange | null {
    if (envelope.commandName !== 'project.invite' && envelope.commandName !== 'project.invitation.revoke') return null;
    this.host.guard();
    this.may('project.share');
    const earlier = this.log.get(envelope.idempotencyKey);
    // The stored answer has no token: a retry does not show the link again.
    if (earlier) return { ...earlier, replayed: true };
    const result = envelope.commandName === 'project.invite' ? this.invite(envelope.input as ProjectInvite) : this.revoke(envelope.input as InvitationRevoke);
    this.log.set(envelope.idempotencyKey, { invitation: result.invitation, replayed: false });
    if (this.loseNextAnswer && envelope.commandName === 'project.invite') {
      this.loseNextAnswer = false;
      throw new ApiFailure(0, { error: 'network' }, 'Yanıt kayboldu.');
    }
    return result;
  }

  private invite(input: ProjectInvite): InvitationChange {
    const email = input.email.trim().toLowerCase();
    const [local, domain] = [email.slice(0, email.indexOf('@')), email.slice(email.indexOf('@') + 1)];
    if (!local || email.indexOf('@') < 0 || domain.includes('@') || !domain.includes('.') || domain.startsWith('.') || domain.endsWith('.') || /\s/.test(email))
      throw invalid('Davet için geçerli bir e-posta adresi yazın (ör. ad@kurum.gov.tr).', 'email');
    if (input.role === 'manager') throw invalid('Davetle en çok düzenleyici rolü verilir; yöneticilik kurum üyelerine paylaşımla verilir.', 'role');
    const now = Date.now();
    const expires = input.expiresAt ? Date.parse(input.expiresAt) : now + 14 * DAY_MS;
    if (Number.isNaN(expires)) throw invalid(`expiresAt bir RFC 3339 zamanı olmalı: ${input.expiresAt}`, 'expiresAt');
    if (expires <= now || expires > now + 90 * DAY_MS) throw invalid('Davetin bitişi gelecekte ve en çok 90 gün sonra olmalı.', 'expiresAt');
    // One waiting invitation per e-mail: a new one replaces it (its link stops working).
    for (const i of this.invitations) if (i.email === email && i.state === 'pending') i.state = 'revoked';
    const token = [...crypto.getRandomValues(new Uint8Array(32))].map((b) => b.toString(16).padStart(2, '0')).join('');
    const invitation = {
      id: crypto.randomUUID(),
      email,
      role: input.role,
      state: 'pending' as const,
      createdBy: this.host.userId,
      createdByName: 'Ayşe Yılmaz',
      // Distinct, rising times: the list's order is the order they were made.
      createdAt: new Date(Date.parse('2026-09-26T10:00:00Z') + ++this.clock * 1000).toISOString(),
      expiresAt: new Date(expires).toISOString(),
      token,
    };
    this.invitations.push(invitation);
    return { invitation: this.view(invitation), token, replayed: false };
  }

  private revoke(input: InvitationRevoke): InvitationChange {
    const i = this.invitations.find((x) => x.id === input.invitationId && x.state === 'pending');
    if (!i) throw missing();
    i.state = 'revoked';
    return { invitation: this.view(i), replayed: false };
  }

  private view(i: ProjectInvitation & { token: string }): ProjectInvitation {
    const { token: _token, ...shown } = i;
    return { ...shown };
  }

  /** `GET …/invitations`: newest first, never a token. */
  list(): ProjectInvitations {
    this.host.guard();
    this.may('project.share');
    return { invitations: [...this.invitations].reverse().map((i) => this.view(i)) };
  }

  /** `POST /v1/invitations/accept`, by `taker`. */
  accept(token: string): InvitationAccepted {
    const t = this.taker;
    if (!t) throw new ApiFailure(401, { error: 'unauthenticated', message: 'Oturum açın.' }, 'Oturum yok.');
    const inv = this.invitations.find((i) => i.token === token.trim().toLowerCase());
    const notFound = () =>
      new ApiFailure(404, { error: 'not_found', message: 'Davet bulunamadı: süresi dolmuş, kullanılmış ya da geri alınmış olabilir. Davet edenden yeni bir bağlantı isteyin.' }, 'Yok.');
    if (!inv || inv.state !== 'pending' || Date.parse(inv.expiresAt) <= Date.now()) throw notFound();
    const forbidden = (message: string) => new ApiFailure(403, { error: 'forbidden', message }, message);
    if (!t.email || t.email.toLowerCase() !== inv.email) throw forbidden('Bu davet başka bir e-posta adresi için. Davetin gönderildiği adresin hesabıyla giriş yapın.');
    if (!t.verified)
      throw forbidden('Hesabınızın e-posta adresi doğrulanmamış. Adresinizi kimlik sağlayıcınızda doğrulayıp yeniden giriş yapın; yerel hesapta kurum yöneticinize başvurun.');
    if (this.host.deleted())
      throw new ApiFailure(410, { error: 'project_deleted', message: 'Davet edilen proje silinmiş ya da kurumu etkin değil; davet edenle görüşün.' }, 'Silindi.');
    const answer = (role: InvitationAccepted['role'], guest: boolean): InvitationAccepted => {
      Object.assign(inv, { state: 'accepted', acceptedByName: t.name, acceptedAt: '2026-09-26T12:00:00Z' });
      return { tenantId: 't', tenantName: 'Büro', tenantKind: this.tenantKind, projectId: 'p', projectName: this.host.projectName, role, guest };
    };
    if (t.owner) return answer('owner', false);
    let guest = false;
    if (this.tenantKind === 'organization') {
      if (t.member === 'inactive') throw forbidden('Bu kurumdaki üyeliğiniz ya da koltuğunuz etkin değil; kurum yöneticinize başvurun.');
      if (!t.member) {
        if (!this.guestsAllowed) throw forbidden('Bu kurum dışarıdan misafir kabul etmiyor; kurum yöneticisiyle görüşün.');
        guest = true;
      }
    }
    // A stronger role held already stays; a guest's is at most an editor's.
    let given: GrantRole = t.holds && RANK.indexOf(t.holds) > RANK.indexOf(inv.role) ? t.holds : inv.role;
    if (guest && given === 'manager') given = 'editor';
    this.granted.set(t.id, { role: given, guest });
    return answer(given, guest);
  }
}
