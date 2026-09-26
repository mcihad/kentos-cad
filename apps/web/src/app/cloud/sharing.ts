import type { AccessBlock } from '../../contracts/generated/AccessBlock';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { GrantRole } from '../../contracts/generated/GrantRole';
import type { ProjectAccessHolder } from '../../contracts/generated/ProjectAccessHolder';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectList } from '../../contracts/generated/ProjectList';
import type { ProjectRole } from '../../contracts/generated/ProjectRole';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { ApiFailure } from './api';

/**
 * Sharing a cloud project as the share dialog does it (docs/adr/0015,
 * TODOS.md CLOUD-16, CLOUD-21): the product commands `project.share` and
 * `project.access.revoke`, and the words the interface uses for roles, for
 * where a person's access comes from and for why someone cannot use a
 * project. The server decides every request; the dialog only shows what it
 * answers and enables what the account's own permissions allow.
 */

export const ROLE_LABEL: Record<ProjectRole, string> = {
  owner: 'Sahip',
  manager: 'Yönetici',
  editor: 'Düzenleyici',
  commenter: 'Yorumcu',
  viewer: 'Görüntüleyici',
};

/** The roles a share gives, weakest first (ownership is transferred, not shared). */
export const GRANT_ROLES: readonly GrantRole[] = ['viewer', 'commenter', 'editor', 'manager'];

/** What each role may do, in one line (docs/adr/0015's table). */
export const ROLE_HINT: Record<GrantRole, string> = {
  viewer: 'Projeyi açar, görür, indirir ve geçmişine bakar; değiştiremez.',
  commenter: 'Görüntüleyicinin yapabildikleri ve yorum.',
  editor: 'Nesneleri çizer, değiştirir ve siler.',
  manager: 'Düzenleyicinin yapabildikleri; ad, ayar ve katmanları değiştirir, projeyi paylaşır.',
};

export const BLOCK_TEXT: Record<AccessBlock, string> = {
  expired: 'Paylaşımın süresi doldu',
  notMember: 'Kurumun üyesi değil',
  inactive: 'Hesabı ya da kurum üyeliği etkin değil',
  noSeat: 'Kurumda koltuğu yok',
  guestsOff: 'Misafir; kurum dışarıdan misafir kabul etmiyor',
};

export const STORAGE_TEXT: Record<ProjectStorage, { title: string; detail: string }> = {
  database: {
    title: 'Yönetilen PostGIS veritabanı',
    detail: 'Nesneler sunucudaki veritabanında tek tek saklanır; her kayıt tek işlemde yazılır ve erişimi olan herkes hemen görür. Paylaşım alıcıya veritabanı hesabı ya da parolası vermez.',
  },
  file: {
    title: 'Dosya (KCAD revizyonları)',
    detail: 'Proje sunucuda değişmez .kcad revizyonları olarak saklanır; her kayıt yeni bir revizyondur ve dayandığı revizyonla karşılaştırılır, arada başkası kaydettiyse üzerine yazılmaz. Paylaşım alıcıya dosya deposuna ayrı bir erişim vermez.',
  },
};

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

/** `project.share` v1: gives `userId` a role, or changes theirs; `expiresAt` (RFC 3339) ends it by itself. */
export function shareEnvelope(tenantId: string, projectId: string, userId: string, role: GrantRole, expiresAt?: string): CommandEnvelope {
  return envelope('project.share', tenantId, projectId, expiresAt ? { userId, role, expiresAt } : { userId, role });
}

/** `project.access.revoke` v1: takes `userId`'s grant away. */
export function revokeEnvelope(tenantId: string, projectId: string, userId: string): CommandEnvelope {
  return envelope('project.access.revoke', tenantId, projectId, { userId });
}

/** A date as the interface writes it: 31.12.2026. */
export const dateText = (iso: string): string => new Date(iso).toLocaleDateString('tr-TR', { day: '2-digit', month: '2-digit', year: 'numeric' });

/** The end of a day picked in a date field (`YYYY-MM-DD`), in local time, for `expiresAt`; null for none or nonsense. */
export function endOfDay(date: string): string | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return null;
  const t = new Date(`${date}T23:59:59`);
  return Number.isNaN(t.getTime()) ? null : t.toISOString();
}

/** One row of the share dialog's list. */
export interface PersonRow {
  userId: string;
  name: string;
  email?: string;
  /** The role now, or “Erişemiyor”. */
  roleLabel: string;
  /** Where it comes from, or why not. */
  source: string;
  blocked: boolean;
  /** The signed-in account itself (its own access does not change here). */
  you: boolean;
  owner: boolean;
  grant?: GrantRole;
  expiresAt?: string;
  /** A guest's grant (docs/adr/0035): outside the organisation, its role came by invitation. */
  guest: boolean;
  /**
   * Its grant's role can be changed here: an unexpired grant of someone
   * else, and the account may share. Not a guest's: an organisation shares
   * with its members only, so a guest's role changes by a new invitation.
   */
  canChange: boolean;
  /** Its grant can be taken away here. */
  canRevoke: boolean;
}

/** Where a person's access comes from, or why they cannot use the project, in one line. */
export function sourceText(p: ProjectAccessHolder): string {
  const grant = p.grant ? `paylaşım: ${ROLE_LABEL[p.grant]}${p.expiresAt ? `, ${dateText(p.expiresAt)} tarihine kadar` : ''}` : '';
  if (p.blocked) return [BLOCK_TEXT[p.blocked], grant].filter(Boolean).join(' · ');
  switch (p.via) {
    case 'owner':
      return 'Proje sahibi';
    case 'policy':
      return ['Kurum politikası: kurum yöneticisi', grant].filter(Boolean).join(' · ');
    default: {
      // A guest: outside the organisation, through an accepted invitation (docs/adr/0035).
      const kind = p.guest ? 'Misafir (davetle)' : 'Paylaşım';
      return p.expiresAt ? `${kind}, ${dateText(p.expiresAt)} tarihine kadar` : kind;
    }
  }
}

/** The people of a project as the share dialog lists them: the owner first, then by name. */
export function personRows(list: ProjectAccessList, me: string, mayShare: boolean): PersonRow[] {
  const rows = list.people.map((p): PersonRow => {
    const you = p.userId === me;
    const owner = p.userId === list.ownerId;
    const other = mayShare && !you && !owner && !!p.grant;
    return {
      userId: p.userId,
      name: p.displayName || 'Adı görünmeyen hesap',
      email: p.email,
      roleLabel: p.role ? ROLE_LABEL[p.role] : 'Erişemiyor',
      source: sourceText(p),
      blocked: !p.role,
      you,
      owner,
      grant: p.grant,
      expiresAt: p.expiresAt,
      guest: p.guest,
      // Changing an ended grant's role would give it back without an end: the person is shared with again instead.
      canChange: other && !p.expired && p.via === 'grant' && !p.guest,
      canRevoke: other,
    };
  });
  return rows.sort((a, b) => Number(b.owner) - Number(a.owner) || a.name.localeCompare(b.name, 'tr'));
}

/** “Benimle paylaşılanlar”: the projects shared with this account (a grant), newest first. */
export function sharedWithMe(list: ProjectList): ProjectSummary[] {
  return list.projects.filter((p) => p.access.via === 'grant').sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt));
}

/**
 * A failed request as the dialog says it: what failed (`failed`, e.g.
 * “Paylaşılamadı”), then the cause and what to do, in the server's own words
 * where it has them.
 */
export function failureText(e: unknown, failed: string): string {
  if (e instanceof ApiFailure) {
    if (e.code === 'network') return `${failed}: sunucuya ulaşılamadı. Bağlantınızı denetleyip yeniden deneyin.`;
    if (e.status === 401) return `${failed}: oturumunuz sona erdi. Yeniden giriş yapıp tekrar deneyin.`;
    if (e.notFound && e.message === 'Proje bulunamadı.')
      return `${failed}: proje bulunamadı; silinmiş ya da size erişimi kaldırılmış olabilir. Pencereyi kapatıp proje listesini yenileyin.`;
    if (e.transient) return `${failed}: sunucu şu an yanıt vermiyor. Birazdan yeniden deneyin.`;
    return `${failed}: ${e.message}`;
  }
  return `${failed}: ${e instanceof Error ? e.message : String(e)}`;
}
