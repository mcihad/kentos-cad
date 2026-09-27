import type { MembershipView } from '../../contracts/generated/MembershipView';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import { ApiFailure } from './api';
import type { MetadataPatch } from './lifecycle';
import { workspaceName } from './session';

/**
 * The catalog's project forms (docs/adr/0028, 0039; ui/cloud/ProjectForms.ts
 * and ProjectActions.ts): Proje bilgileri, Yeniden adlandır, Kopyasını
 * oluştur, and a new project in the other storage mode. What each says,
 * which workspaces it offers, when its button is on, what it sends and the
 * lines it writes, apart from the DOM. The server checks and normalizes
 * every value; the forms only offer them. fixtures/cloud/v1/forms.json holds
 * these for the desktop (format in fixtures/cloud/README.md).
 */

/** Names are at most this long, descriptions this long (the fields' limits; the server's too). */
export const NAME_MAX = 200;
export const DESCRIPTION_MAX = 2000;

const NO_PLACE = 'Proje açabileceğiniz bir çalışma alanınız yok (project.create); kurum yöneticinize başvurun.';

export const FORM_TEXTS = {
  cancel: 'Vazgeç',
  saving: 'Kaydediliyor…',
  place: 'Çalışma alanı',
  noPlace: NO_PLACE,
  fields: {
    type: 'Tür',
    typeLabel: 'Proje türü',
    description: 'Açıklama',
    descriptionPlaceholder: 'İsteğe bağlı: işin konusu, yeri, dayanağı',
    tags: 'Etiketler',
    tagsPlaceholder: 'Virgülle ayırın: Kadıköy, 2026',
  },
  metadata: {
    title: 'Proje bilgileri',
    name: 'Proje adı',
    save: 'Kaydet',
    saved: (name: string) => `“${name}” projesinin bilgileri kaydedildi.`,
  },
  rename: {
    title: 'Bulut projesini yeniden adlandır',
    name: 'Yeni ad',
    hint: 'Projeye erişimi olan herkes yeni adı görür.',
    save: 'Yeniden adlandır',
    saved: (name: string) => `Proje “${name}” olarak yeniden adlandırıldı.`,
    waiting: (name: string) => `Yeni ad (“${name}”) bu cihazda bekliyor; sunucuya ulaşılınca kaydedilir.`,
  },
  duplicate: {
    title: 'Projenin kopyasını oluştur',
    lead: (name: string) => `“${name}” yeni bir proje olarak kopyalanır.`,
    consequences: [
      'Katmanlar, ayarlar, stiller, açıklama, tür, etiketler ve bütün nesneler kalıcı kimlikleriyle kopyalanır.',
      'Geçmiş (komut günlüğü, olaylar), paylaşımlar ve favoriler kopyalanmaz; arşivlenmiş proje etkin bir kopya olur.',
      'Kopya sizin olur: siz paylaşana kadar yalnız size ve kurum politikasıyla kurum yöneticilerine görünür.',
    ],
    name: 'Kopyanın adı',
    nameLabel: 'Kopyanın adı',
    placeLabel: 'Kopyanın çalışma alanı',
    make: 'Kopyasını oluştur',
    running: 'Kopyalanıyor…',
    done: (name: string, objects: string) => `“${name}” oluşturuldu: ${countText(objects)} nesne kopyalandı.`,
  },
  convert: {
    name: 'Yeni projenin adı (isteğe bağlı)',
    nameLabel: 'Yeni projenin adı',
    placeLabel: 'Yeni projenin çalışma alanı',
  },
} as const;

/**
 * A count the server sends as decimal text, grouped as Turkish writes it
 * (1.234.567), without going through a JavaScript number; anything else as
 * it came.
 */
export const countText = (objects: string): string => (/^\d+$/.test(objects) ? objects.replace(/\B(?=(\d{3})+$)/g, '.') : objects);

/** The name a copy is offered. */
export const copyName = (name: string): string => `${name} (kopya)`;

/** What the metadata form changes, field by field, of the catalog version shown: only what differs (the name trimmed, the tags in order). */
export function metadataPatch(
  shown: { name: string; projectType: ProjectType; description: string; tags: readonly string[] },
  now: { name: string; projectType: ProjectType; description: string; tags: readonly string[] },
): MetadataPatch {
  const out: MetadataPatch = {};
  if (now.name.trim() !== shown.name) out.name = now.name.trim();
  if (now.projectType !== shown.projectType) out.projectType = now.projectType;
  if (now.description !== shown.description) out.description = now.description;
  if (now.tags.join('\n') !== shown.tags.join('\n')) out.tags = [...now.tags];
  return out;
}

/** Kaydet is on while the name is not empty and something changed. */
export const metadataSavable = (name: string, patch: MetadataPatch): boolean => !!name.trim() && Object.keys(patch).length > 0;

/** Yeniden adlandır is on while the new name is not empty and not the name it has. */
export function renameSavable(typed: string, current: string): boolean {
  const v = typed.trim();
  return !!v && v !== current;
}

/** Kopyasını oluştur is on while there is a workspace for it and a name. */
export const duplicateSavable = (places: number, name: string): boolean => places > 0 && !!name.trim();

/** A workspace a form offers: its tenant and its name in the list (“Kişisel” for the account's own space). */
export interface Place {
  tenantId: string;
  label: string;
}

/**
 * The workspaces the account may open projects in (active, with a seat,
 * `project.create`), the source's first, then the rest in the account's
 * order; the list is off while it offers fewer than two.
 */
export function creatablePlaces(memberships: readonly MembershipView[], first: string): Place[] {
  return memberships
    .filter((m) => m.active && m.seat && m.capabilities.includes('project.create'))
    .sort((a, b) => Number(b.tenantId === first) - Number(a.tenantId === first))
    .map((m) => ({ tenantId: m.tenantId, label: workspaceName(m.tenantKind, m.tenantName, true) }));
}

/** What the form converting a project to the other storage mode says (docs/adr/0039). */
export interface ConvertForm {
  to: ProjectStorage;
  title: string;
  lead: string;
  consequences: string[];
  placeholder: string;
  running: string;
}

/**
 * “PostGIS'e aktar” (a file project) or “Dosya projesine çevir” (a database
 * project). `openDirty`: this project is the one open here, with unsaved
 * changes (they do not go to the new database project).
 */
export function convertForm(p: { name: string; storage: ProjectStorage }, openDirty: boolean): ConvertForm {
  const to: ProjectStorage = p.storage === 'file' ? 'database' : 'file';
  const db = to === 'database';
  const consequences = [
    db
      ? 'Projenin en yeni revizyonu aktarılır: her nesne kalıcı kimliğiyle; ayarlar, katmanlar ve stiller dosyadan. Analitik CAD tanımları korunur, GIS çizgisine indirgenmez.'
      : 'Projenin şimdiki hâli (tek anlık görüntüsü) yeni dosya projesinin 1. revizyonu olur.',
    `“${p.name}” olduğu gibi kalır: aynı çizimin iki yazılabilir sahibi olmaz, yeni proje başka bir projedir.`,
    'Yeni proje sizin olur; geçmiş, paylaşım ve favoriler gelmez. Açıklama, tür ve etiketler gelir.',
  ];
  if (db) consequences.push('Sunucunun almadığı bir nesne (±1 000 000 000 sınırını aşan değer) varsa hiçbir proje oluşturulmaz ve nesne söylenir.');
  if (db && openDirty) consequences.push('Açık çizimdeki kaydedilmemiş değişiklikler aktarılmaz: önce Kaydet ile yeni revizyon yazın.');
  return {
    to,
    title: db ? "PostGIS'e aktar" : 'Dosya projesine çevir',
    lead: `“${p.name}” projesinden ${db ? 'nesne nesne veritabanında saklanan' : 'dosya olarak (KCAD revizyonları) saklanan'} yeni bir proje oluşturulur.`,
    consequences,
    placeholder: `${p.name} (${db ? 'PostGIS' : 'dosya'})`,
    running: db ? 'Veritabanına aktarılıyor…' : 'Dosya projesi oluşturuluyor…',
  };
}

/** The log's line for a new project made in the other storage mode (`objects`: the server's decimal text). */
export const convertedLine = (name: string, objects: string, to: ProjectStorage): string =>
  `“${name}” oluşturuldu: ${countText(objects)} nesne${to === 'database' ? ' veritabanına aktarıldı' : ', 1. revizyon'}.`;

/** A failed request as a sentence: the server's words, or what to do when it gave none. */
export function failureReason(e: unknown): string {
  if (e instanceof ApiFailure && e.code === 'conflict') return 'Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.';
  if (e instanceof ApiFailure && e.code === 'network') return 'Sunucuya ulaşılamadı; bağlantınızı denetleyip yeniden deneyin.';
  return e instanceof Error ? e.message : 'İşlem tamamlanamadı.';
}

/** Why a conversion was refused: an object the server did not take is named by its place in the file (`entities[i]` in the error's path). */
export function convertReason(e: unknown): string {
  const m = e instanceof Error ? /^entities\[(\d+)\]/.exec((e as { path?: string }).path ?? '') : null;
  return m ? `${failureReason(e)} (dosyanın ${Number(m[1]) + 1}. nesnesi; hiçbir proje oluşturulmadı).` : failureReason(e);
}
