import type { GrantRole } from '../../contracts/generated/GrantRole';
import type { InvitationChange } from '../../contracts/generated/InvitationChange';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectInvitation } from '../../contracts/generated/ProjectInvitation';
import type { ProjectRole } from '../../contracts/generated/ProjectRole';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { TenantKind } from '../../contracts/generated/TenantKind';
import { INVITE_DAYS, emailProblem, type inviteQuestion } from './invitations';
import { ROLE_LABEL, STORAGE_TEXT, dateText, personRows, type PersonRow } from './sharing';

/**
 * The share dialog's words and small rules (ui/cloud/ShareDialog.ts,
 * shareFind.ts and shareInvites.ts), apart from its DOM: what each part
 * says and offers, and why a control is off. The roles, who may change
 * what and the invitation rules are sharing.ts's and invitations.ts's.
 * fixtures/cloud/v1/share.json holds all of them for the desktop's
 * Paylaş window (format in fixtures/cloud/README.md).
 */

export const SHARE_TEXTS = {
  title: 'Projeyi paylaş',
  tabs: { label: 'Paylaşım', people: 'Kişiler', invites: 'Davetler' },
  close: 'Kapat',
  people: {
    add: 'Kişi ekle',
    role: 'Rol',
    until: 'Bitiş (isteğe bağlı)',
    untilLabel: 'Bitiş tarihi (isteğe bağlı)',
    share: 'Paylaş',
    title: 'Erişimi olanlar',
    listLabel: 'Erişimi olanlar',
    loading: 'Erişimi olanlar yükleniyor…',
    you: ' (siz)',
    remove: 'Kaldır',
    removeLabel: (name: string) => `${name} erişimini kaldır`,
    roleLabel: (name: string) => `${name} için rol`,
    count: (n: number) => `${n} kişi erişebiliyor`,
    adding: (name: string) => `${name} ekleniyor…`,
    changing: (name: string) => `${name} için rol değiştiriliyor…`,
    revoking: (name: string) => `${name} için erişim kaldırılıyor…`,
    badDate: 'Bitiş tarihi okunamadı: takvimden bir gün seçin ya da alanı boş bırakın.',
    readFailed: 'Erişim listesi okunamadı',
    shareFailed: 'Paylaşılamadı',
    changeFailed: 'Rol değiştirilemedi',
    revokeFailed: 'Erişim kaldırılamadı',
    guestRole: 'Misafirin rolü davetle verilir. Değiştirmek için erişimini kaldırıp yeni rolle yeniden davet edin.',
    storage: (title: string) => `Saklama: ${title}. `,
  },
  find: {
    placeholder: 'Ad ya da e-posta yazın',
    label: 'Paylaşılacak kişi',
    list: 'Bulunan kişiler',
    has: (role: string) => `şu an ${role}`,
    failed: 'Kişi aranamadı',
  },
  invites: {
    email: 'E-postayla davet et',
    emailLabel: 'Davet edilecek e-posta',
    placeholder: 'ad@kurum.gov.tr',
    role: 'Rol',
    roleLabel: 'Davetin rolü',
    wait: 'Geçerlilik',
    waitLabel: 'Davetin geçerlilik süresi',
    send: 'Davet et',
    title: 'Davetler',
    listLabel: 'Davetler',
    loading: 'Davetler yükleniyor…',
    empty: 'Bekleyen ya da son 30 günde sonuçlanmış davet yok. Kurum dışından biriyle çalışmak için yukarıdan e-postayla davet edin.',
    noRight: 'Davetleri görmek ve göndermek için bu projede paylaşım yetkiniz olmalı (project.share).',
    count: (waiting: number) => `${waiting} bekliyor`,
    revoke: 'Geri al',
    revokeLabel: (email: string) => `${email} davetini geri al`,
    inviting: (email: string) => `“${email}” davet ediliyor…`,
    revoking: (email: string) => `“${email}” için davet geri alınıyor…`,
    created: 'Davet oluşturuldu. Bağlantıyı kopyalayıp davet ettiğiniz kişiye iletin.',
    readFailed: 'Davetler okunamadı',
    sendFailed: 'Davet gönderilemedi',
    revokeFailed: 'Davet geri alınamadı',
    linkLabel: 'Davet bağlantısı',
    copy: 'Kopyala',
    copied: 'Kopyalandı',
    copiedSay: 'Bağlantı panoya kopyalandı; davet ettiğiniz kişiye iletin.',
    copyFailed: 'Bağlantı panoya kopyalanamadı: alandaki bağlantı seçildi, Ctrl+C ile kopyalayın.',
    noLink: 'Ancak bağlantı bu yanıtta yok; sunucu onu yalnız ilk yanıtta verir.',
    noLinkWarn: 'Bağlantıyı almak için aynı adrese yeniden davet gönderin; bu davetin bağlantısı artık çalışmaz.',
  },
} as const;

/** The avatar's letters: the first two words' initials, or “?”. */
export const initials = (name: string): string =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]!.toLocaleUpperCase('tr'))
    .join('') || '?';

/** Why Paylaş is off, or '' when it is on: no right to share, or nobody picked yet. */
export function shareTip(mayShare: boolean, chosen: boolean): string {
  if (!mayShare) return 'Bu projede paylaşım yetkiniz yok (project.share).';
  return chosen ? '' : 'Önce “Kişi ekle” alanında bir kişi arayıp listeden seçin.';
}

/** What a share did: nothing new, a role changed (the person had a grant), or someone added. */
export function sharedText(name: string, role: GrantRole, changed: boolean, had: boolean): string {
  if (!changed) return `${name} zaten ${ROLE_LABEL[role]} rolündeydi; değişen bir şey yok.`;
  return had ? `${name} artık ${ROLE_LABEL[role]}.` : `${name} projeye ${ROLE_LABEL[role]} olarak eklendi.`;
}

/** A grant's role changed in its row. */
export const roleChangedText = (name: string, role: GrantRole): string => `${name} artık ${ROLE_LABEL[role]}.`;

/** A grant taken away. */
export const revokedText = (name: string): string => `${name} artık projeye erişemiyor.`;

/** The question before taking someone's access away: a guest comes back by invitation, a member by a new share. */
export function revokeQuestion(name: string, project: string, guest: boolean): { title: string; message: string; details: string[]; action: string } {
  return {
    title: 'Erişimi kaldır',
    message: `${name}, “${project}” projesine artık erişemesin mi?`,
    details: [
      'Projeyi şu anda açık tutuyorsa kaydı hemen durur; gönderilmemiş değişiklikleri kendi cihazında kalır.',
      'Daha önce indirdiği kopyalar ve ekranında gördükleri geri alınamaz.',
      guest ? 'Kurum dışından olduğu için erişimini yeniden davetle geri verebilirsiniz.' : 'Yeniden paylaşarak erişimini geri verebilirsiniz.',
    ],
    action: 'Erişimi kaldır',
  };
}

/** Why a row's access cannot be taken away here (the lock in its place), for a row that offers no Kaldır. */
export function fixedReason(row: Pick<PersonRow, 'you' | 'owner' | 'grant'>): string {
  if (row.you) return 'Kendi erişiminizi buradan değiştiremezsiniz; proje sahibine ya da başka bir yöneticiye başvurun.';
  if (row.owner) return 'Proje sahibinin erişimi paylaşımla değişmez.';
  if (!row.grant) return 'Kurum politikasından gelen erişim paylaşımla değişmez.';
  return 'Bu erişimi değiştirme yetkiniz yok.';
}

const OUTSIDE = 'Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur.';

/** The note under the people: who else can reach the project, by the workspace and the organisation's policy. */
export function policyText(tenantKind: TenantKind, adminsAccessAllProjects: boolean): string {
  if (tenantKind === 'personal') return 'Kişisel alanınızdaki bu proje yalnız paylaştığınız ve davet ettiğiniz kişilere açıktır.';
  return adminsAccessAllProjects
    ? `Kurumun politikası açık: kurum sahibi ve yöneticileri paylaşılmamış kurum projelerine de yönetici olarak erişir. Paylaşım kurumun üyeleriyledir; ${OUTSIDE}`
    : `Kurumun politikası kapalı: kurum yöneticileri de yalnız kendileriyle paylaşılan projelere erişir. Paylaşım kurumun üyeleriyledir; ${OUTSIDE}`;
}

/** One person's row as the list draws it: the row's rule, and the words of its fixed parts. */
export interface PersonView extends PersonRow {
  /** The avatar's letters. */
  initials: string;
  /** Under the name: the e-mail and where the access comes from. */
  sub: string;
  /** The role cannot be changed here because it is a guest's: the role's tip. */
  roleTip: string | null;
  /** No Kaldır: why (the lock's tip). */
  fixed: string | null;
}

/** The Kişiler tab once the server has listed the people: where the project keeps its content, the rows, the count and the note. */
export interface PeopleView {
  storage: { lead: string; detail: string };
  rows: PersonView[];
  count: string;
  policy: string;
}

export function peopleView(list: ProjectAccessList, me: string, mayShare: boolean): PeopleView {
  const text = STORAGE_TEXT[list.storage];
  const rows = personRows(list, me, mayShare).map(
    (r): PersonView => ({
      ...r,
      initials: initials(r.name),
      sub: [r.email, r.source].filter(Boolean).join(' · '),
      roleTip: !(r.canChange && r.grant) && r.guest ? SHARE_TEXTS.people.guestRole : null,
      fixed: r.canRevoke ? null : fixedReason(r),
    }),
  );
  return {
    storage: { lead: SHARE_TEXTS.people.storage(text.title), detail: text.detail },
    rows,
    count: SHARE_TEXTS.people.count(rows.filter((r) => !r.blocked).length),
    policy: policyText(list.tenantKind, list.adminsAccessAllProjects),
  };
}

/** The storage line alone (the dialog shows it before the list is drawn again). */
export const storageLine = (storage: ProjectStorage): { lead: string; detail: string } => ({ lead: SHARE_TEXTS.people.storage(STORAGE_TEXT[storage].title), detail: STORAGE_TEXT[storage].detail });

// ── Kişi ekle: the finder's list ─────────────────────────────────────

/**
 * Nobody found for `query`: what the workspace allows, and, for a whole
 * e-mail, the offer to invite it instead (`invite`; null when it is no
 * address the server takes).
 */
export function nobodyFound(query: string, personal: boolean): { text: string; invite: string | null } {
  const text = personal
    ? `“${query}” ile eşleşen kimse yok. Kurumlarınızın dışından biriyle “Davetler”den e-postayla paylaşabilirsiniz.`
    : `“${query}” ile eşleşen etkin bir kurum üyesi yok. Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur.`;
  return { text, invite: emailProblem(query) ? null : `“${query.trim()}” adresine e-postayla davet gönder…` };
}

/** A found person who can use the project already: their role now, beside their name. */
export const candidateNote = (role: ProjectRole | undefined): string | null => (role ? SHARE_TEXTS.find.has(ROLE_LABEL[role]) : null);

/** The finder searches once this much is typed, spaces aside. */
export const searchesFor = (query: string): boolean => query.trim().replace(/\s+/g, '').length >= 2;

// ── Davetler ─────────────────────────────────────────────────────────

/** A wait's name in the Geçerlilik list. */
export const dayText = (days: number): string => (days === INVITE_DAYS ? `${days} gün (varsayılan)` : `${days} gün`);

/** Why Davet et is off, or '' when it is on. */
export function inviteTip(mayShare: boolean, email: string): string {
  if (!mayShare) return 'Bu projede davet yetkiniz yok (project.share).';
  return email.trim() ? '' : 'Önce davet edilecek e-posta adresini yazın.';
}

/** The count beside “Davetler”: the waiting ones, nothing while the list is empty. */
export const invitesCount = (list: readonly ProjectInvitation[]): string => (list.length ? SHARE_TEXTS.invites.count(list.filter((i) => i.state === 'pending').length) : '');

/** The note under the invitations: what the invited get, by the workspace. */
export function inviteRules(personal: boolean): string {
  return personal
    ? 'Davet edilen, bağlantıyı açıp davetin gönderildiği e-postanın hesabıyla girince projeye paylaşımla erişir. Bağlantıyı siz iletirsiniz; KentOS e-posta göndermez.'
    : 'Davet edilen, bağlantıyı açıp davetin gönderildiği e-postanın hesabıyla girince projeye erişir: kurumun üyesiyse paylaşımla, değilse misafir olarak (yalnız bu projeyi görür; rolü en çok Düzenleyici; kurum misafir almıyorsa kabul edilmez). Bağlantıyı siz iletirsiniz; KentOS e-posta göndermez.';
}

/**
 * What the panel says of a new invitation: who was invited as what until
 * when; with the link (shown once) the warning that it is shown only now,
 * without it (a retry's stored answer) how to get one.
 */
export function invitedLink(change: InvitationChange, days: number): { what: string; warn: string; link: boolean } {
  const i = change.invitation;
  const what = `“${i.email}” davet edildi: ${ROLE_LABEL[i.role]}, ${dateText(i.expiresAt)} tarihine kadar bekler.`;
  if (!change.token) return { what: `${what} ${SHARE_TEXTS.invites.noLink}`, warn: SHARE_TEXTS.invites.noLinkWarn, link: false };
  return {
    what,
    warn: `Bu bağlantı yalnız şimdi gösterilir: sunucu onu saklamaz, pencere kapanınca yeniden gösterilemez. Kopyalayıp davet ettiğiniz kişiye kendiniz iletin; ${days} gün içinde, bir kez kullanılabilir. Kaybederseniz yeniden davet edin (eski bağlantı çalışmaz olur).`,
    link: true,
  };
}

/** The question before inviting an address that has a waiting invitation or can use the project already (`inviteQuestion`'s answer). */
export function inviteAsk(address: string, q: NonNullable<ReturnType<typeof inviteQuestion>>): { title: string; message: string; details: string[]; go: string; cancel: string } {
  const details: string[] = [];
  if (q.waiting) details.push(`${dateText(q.waiting.createdAt)} tarihli bekleyen davet geri alınır; onun bağlantısı artık çalışmaz. Yeni bağlantıyı yeniden iletmeniz gerekir.`);
  if (q.holder) details.push(`${q.holder.name} projeye zaten ${q.holder.role} olarak erişebiliyor. Davet ancak daha güçlü bir rol verir; rolü düşürmez.`);
  return { title: q.waiting ? 'Bekleyen davet var' : 'Zaten erişebiliyor', message: `“${address}” için yeni bir davet gönderilsin mi?`, details, go: 'Yeni davet gönder', cancel: 'Vazgeç' };
}

/** The question before withdrawing a waiting invitation. */
export function invitationRevokeQuestion(email: string): { title: string; message: string; details: string[]; action: string; cancel: string } {
  return {
    title: 'Daveti geri al',
    message: `“${email}” adresine gönderilen davet geri alınsın mı?`,
    details: ['Davetin bağlantısı artık çalışmaz; açan kişi projeye erişemez.', 'Kişiye ayrıca haber vermeniz gerekmez; isterseniz daha sonra yeniden davet edebilirsiniz.'],
    action: 'Daveti geri al',
    cancel: 'Vazgeç',
  };
}

/** A change to the people as the log writes it: the project, then what the status line says. */
export const shareLog = (project: string, text: string): string => `“${project}”: ${text}`;

/** The lines the invitations write: to the log (never the link) and to the status line. */
export const INVITE_LINES = {
  invitedLog: (project: string, email: string, role: GrantRole) => `“${project}”: “${email}” ${ROLE_LABEL[role]} olarak davet edildi.`,
  revokedLog: (project: string, email: string) => `“${project}”: “${email}” için davet geri alındı.`,
  revokedSay: (email: string) => `“${email}” için davet geri alındı; bağlantısı artık çalışmaz.`,
};

/** The avatar of an invitation: the address's first letter. */
export const invitationInitial = (email: string): string => email[0]?.toLocaleUpperCase('tr') ?? '?';
