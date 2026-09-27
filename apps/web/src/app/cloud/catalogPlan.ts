import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { STATE_LABEL, TYPE_LABEL, day, when } from './catalog';
import { ROLE_LABEL } from './sharing';

/**
 * What the catalog shows and offers for a project, decided apart from
 * drawing it (docs/adr/0028): the texts of a list's row, the selected
 * project's pane (favourite toggle, chips, tabs, fact rows, actions with
 * the reason each one is off), the window's main button, and the questions
 * and lines of the lifecycle's actions. Pure: the dialog draws it, and
 * fixtures/cloud/v1/catalog.json pins it for both platforms.
 */

/** Why the account may not do something to a project: the right it needs. */
export const deniedText = (name: string, what: string, permission: ProjectPermission): string =>
  `“${name}” projesinde ${what} yetkiniz yok (${permission}); proje sahibine ya da yöneticisine başvurun.`;

/** Why an archived project cannot be changed. */
export const ARCHIVED_TEXT = 'Arşivlenmiş proje değiştirilemez; önce arşivden çıkarın.';

export type DetailAction = 'share' | 'edit' | 'download' | 'duplicate' | 'convert' | 'archive' | 'unarchive' | 'trash' | 'purge';

export interface ActionPlan {
  id: DetailAction;
  label: string;
  icon: string;
  /** Why it is off; null when it can be taken. */
  why: string | null;
  /** A removing action: marked, not amber. */
  danger: boolean;
}

export interface DetailPlan {
  /** The favourite toggle's label; null in the trash. */
  favorite: string | null;
  chips: string[];
  /** The pane's tabs; null in the trash (nothing to show a history of). */
  tabs: readonly string[] | null;
  /** The fact rows shown, in order (their values are drawn by the pane). */
  facts: string[];
  actions: ActionPlan[];
}

const may = (p: ProjectSummary, permission: ProjectPermission) => p.access.permissions.includes(permission);

/** The selected project's pane: `open` when it is the project open here. */
export function detailPlan(p: ProjectSummary, open: boolean): DetailPlan {
  const trashed = p.state === 'trashed';
  const needs = (permission: ProjectPermission, what: string): string | null => (may(p, permission) ? null : deniedText(p.name, what, permission));
  const writable = (permission: ProjectPermission, what: string): string | null => (p.state === 'archived' ? ARCHIVED_TEXT : needs(permission, what));
  const action = (id: DetailAction, label: string, icon: string, why: string | null, danger = false): ActionPlan => ({ id, label, icon, why, danger });
  const toDatabase = p.storage === 'file';
  const actions = trashed
    ? [action('purge', 'Kalıcı olarak sil…', 'trash', needs('project.delete', 'kalıcı olarak silme'), true)]
    : [
        action('share', 'Paylaş…', 'share', needs('project.share', 'paylaşma')),
        action('edit', 'Bilgileri düzenle…', 'edit', writable('project.edit', 'bilgileri değiştirme')),
        action('download', '.kcad olarak indir', 'export', needs('project.download', 'indirme')),
        action('duplicate', 'Kopyasını oluştur…', 'copy', needs('project.download', 'kopyalama')),
        action('convert', toDatabase ? "PostGIS'e aktar…" : 'Dosya projesine çevir…', toDatabase ? 'server' : 'save', needs('project.download', 'dönüştürme')),
        p.state === 'archived'
          ? action('unarchive', 'Arşivden çıkar', 'archive', needs('project.edit', 'arşivden çıkarma'))
          : action('archive', 'Arşivle…', 'archive', needs('project.edit', 'arşivleme')),
        action('trash', 'Çöpe taşı…', 'trash', needs('project.delete', 'çöpe taşıma'), true),
      ];
  const facts = [
    'Çalışma alanı',
    'Sahibi',
    'Rolünüz',
    'Koordinat sistemi',
    'Alan birimi',
    // Nothing of a project in the trash is counted (it does not open).
    ...(trashed ? [] : ['Nesne', 'Kapsam']),
    'Oluşturan',
    'Son değişiklik',
    'Revizyon',
    'Saklama',
    ...(p.archivedAt ? ['Arşivlenme'] : []),
    ...(p.trashedAt ? ['Çöpe taşınma'] : []),
    ...(trashed ? ['Kalıcı silinme'] : []),
  ];
  return {
    favorite: trashed ? null : p.favorite ? 'Favorilerde' : 'Favorilere ekle',
    chips: [TYPE_LABEL[p.projectType], ...(p.state !== 'active' ? [STATE_LABEL[p.state]] : []), ...(open ? ['Şu anda açık'] : [])],
    tabs: trashed ? null : ['Bilgiler', 'Geçmiş'],
    facts,
    actions,
  };
}

export interface PrimaryPlan {
  label: string;
  enabled: boolean;
  /** The button's tip: why it is off, or what it does to this project. */
  why: string;
}

/** The window's one amber button for the list `view` and the selected project. */
export function primaryPlan(view: CatalogView, p: ProjectSummary | null): PrimaryPlan {
  const none = 'Önce listeden bir proje seçin.';
  if (view === 'trash') {
    const ok = !!p && may(p, 'project.delete');
    return { label: 'Geri yükle', enabled: ok, why: !p ? none : ok ? '' : `“${p.name}” projesini geri yükleme yetkiniz yok (project.delete).` };
  }
  return { label: 'Aç', enabled: !!p, why: !p ? none : p.state === 'archived' ? 'Arşivlenmiş proje salt okunur açılır.' : '' };
}

export interface RowPlan {
  /** After the name: the favourite star's tip, “Açık”, “Arşivde”. */
  marks: string[];
  /** Under the name: the type and where (or whose) it is; in the trash, who moved it there. */
  sub: string[];
  /** On the right: the role (shared list), when it goes for good (trash), the time the list is ordered by. */
  side: string[];
}

/** The time a row shows: the one its list is ordered by. */
function timeOf(p: ProjectSummary, sort: CatalogSort): string {
  switch (sort) {
    case 'opened':
      return p.openedAt ? when(p.openedAt) : '';
    case 'created':
      return when(p.createdAt);
    case 'trashed':
      return p.trashedAt ? `Silinme: ${day(p.trashedAt)}` : '';
    default:
      return when(p.updatedAt);
  }
}

/** One project in the list `view` ordered by `sort`; `place` is where it is as the account names it. */
export function rowPlan(p: ProjectSummary, view: CatalogView, sort: CatalogSort, open: boolean, place: string): RowPlan {
  const whose = view === 'organization' || view === 'shared' ? `Sahibi: ${p.ownerName || 'görünmüyor'}` : place;
  return {
    marks: [...(p.favorite ? ['Favorilerinizde'] : []), ...(open ? ['Açık'] : []), ...(p.state === 'archived' && view !== 'archived' ? [STATE_LABEL.archived] : [])],
    sub: view === 'trash' ? [p.trashedByName ? `Çöpe taşıyan: ${p.trashedByName}` : place] : [TYPE_LABEL[p.projectType], whose],
    side: [
      ...(view === 'shared' ? [ROLE_LABEL[p.access.role]] : []),
      ...(view === 'trash' && p.purgeAfter ? [`${day(p.purgeAfter)} tarihinde silinir`] : []),
      timeOf(p, sort),
    ],
  };
}

/** A lifecycle question: its title, question, what it means, and the removing answer. */
export interface Question {
  title: string;
  message: string;
  details: string[];
  action: string;
}

/** Moving to the trash (`project.trash`): `days` how long the trash keeps it, when known; `open` when it is open here. */
export function trashQuestion(name: string, tenant: string, days: number | undefined, open: boolean): Question {
  return {
    title: 'Çöp kutusuna taşı',
    message: `“${name}” projesi (${tenant}) erişimi olan herkes için çöp kutusuna taşınsın mı?`,
    details: [
      'Proje listelerden kalkar; kimse açamaz ve değiştiremez.',
      'Projeyi şu anda açık tutanların kaydı durur; gönderilmemiş değişiklikleri kendi cihazlarında kalır.',
      `Hiçbir şey silinmez: proje sahibi ya da kurum yöneticisi ${days ? `${days} gün` : 'saklama süresi dolana kadar'} içinde Çöp kutusu’ndan geri yükleyebilir; sonra proje kalıcı olarak silinir.`,
      ...(open ? ['Proje şu anda sizde açık: çizim ekranda kalır, dilerseniz yerel bir dosyaya kaydedin.'] : []),
    ],
    action: 'Çöpe taşı',
  };
}

/** Removing a project in the trash for good (`project.purge`). */
export function purgeQuestion(name: string): Question {
  return {
    title: 'Kalıcı olarak sil',
    message: `“${name}” projesi kalıcı olarak silinsin mi? Bu işlem geri alınamaz.`,
    details: [
      'Nesneler, katmanlar, paylaşımlar, komut günlüğü ve olaylar silinir; yalnız kimin ne zaman sildiğini söyleyen denetim kaydı kalır.',
      'Önceden indirilmiş kopyalar ve sunucu yedekleri bu işlemle silinmez.',
    ],
    action: 'Kalıcı olarak sil',
  };
}

/** Archiving (`project.archive`): read-only for everyone until it is unarchived. */
export function archiveQuestion(name: string): Question {
  return {
    title: 'Projeyi arşivle',
    message: `“${name}” projesi arşivlensin mi?`,
    details: [
      'Proje salt okunur olur: nesneleri, adı ve bilgileri değişmez; açılabilir, paylaşımı değiştirilebilir, kopyası oluşturulabilir.',
      'Projeyi şu anda açık tutanların kaydı durur; gönderilmemiş değişiklikleri kendi cihazlarında kalır.',
      'Arşivlenmişler listesinde durur; proje sahibi ya da yöneticisi arşivden çıkarabilir.',
    ],
    action: 'Arşivle',
  };
}

/** The lines the catalog's actions write: to the log (`log`) and under the list (`status`). */
export const CATALOG_LINES = {
  trashed: (name: string, purgeAfter?: string | null) => `“${name}” çöp kutusuna taşındı${purgeAfter ? `; ${day(purgeAfter)} tarihine kadar geri yüklenebilir` : ''}.`,
  trashFailed: (name: string, why: string) => `“${name}” çöp kutusuna taşınamadı: ${why}`,
  trashedStatus: (name: string) => `“${name}” çöp kutusuna taşındı.`,
  purged: (name: string, objects: string | number) => `“${name}” kalıcı olarak silindi (${objects} nesne).`,
  purgeFailed: (name: string, why: string) => `“${name}” kalıcı olarak silinemedi: ${why}`,
  purgedStatus: (name: string) => `“${name}” kalıcı olarak silindi.`,
  archived: (name: string) => `“${name}” arşivlendi.`,
  archiveFailed: (name: string, why: string) => `“${name}” arşivlenemedi: ${why}`,
  archivedStatus: (name: string) => `“${name}” arşivlendi; Arşivlenmişler listesinde duruyor.`,
  unarchived: (name: string) => `“${name}” arşivden çıkarıldı.`,
  unarchivedStatus: (name: string) => `“${name}” arşivden çıkarıldı; yeniden düzenlenebilir.`,
  restored: (name: string) => `“${name}” çöp kutusundan geri yüklendi.`,
  restoredStatus: (name: string) => `“${name}” geri yüklendi; listelerinde yeniden görünür.`,
  favoriteAdded: (name: string) => `“${name}” favorilere eklendi.`,
  favoriteRemoved: (name: string) => `“${name}” favorilerden çıkarıldı.`,
  downloaded: (name: string) => `“${name}” indirildi.`,
} as const;

/** What an empty list says when a search or the type filter is on, and in “Kurum projeleri” without an organisation. */
export const EMPTY_SEARCH = 'Aramanıza uyan proje yok. Başka sözcüklerle ya da tür süzgeci olmadan deneyin.';
export const NO_ORGANIZATION = 'Etkin üyeliğiniz olan bir kurum yok; kurum projeleri burada görünür.';
