import type { FileSaveState } from '../../app/cloud/fileProject';
import { newerTip, type NewerRevision } from '../../app/cloud/fileRevisionsPlan';
import type { LinkState } from '../../app/cloud/socket';
import type { SaveState } from '../../app/cloud/syncCore';
import type { ServerState } from '../../app/server';
import type { Health } from '../../contracts/generated/Health';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';

/**
 * The status bar's cloud cells (cloudCells.ts, StatusBar.ts;
 * CLAUDE.md §21.1, docs/adr/0038): the save cell of an open cloud project,
 * its tip and what a click on it does; the server cell's words and tip; and
 * the account menu behind the server cell, with why an action on the open
 * project is off. Apart from the DOM, as the cells read their state;
 * fixtures/cloud/v1/cells.json holds them for the desktop (format in
 * fixtures/cloud/README.md).
 */

/** A database project's save, as its cell reads it: the state, what waits, the conflicts. */
export interface DatabaseSave {
  kind: 'database';
  state: SaveState;
  pending: number;
  conflicts: number;
}

/** A file project's Kaydet, as its cell reads it (docs/adr/0038). */
export interface FileSave {
  kind: 'file';
  state: FileSaveState;
  /** The revision the drawing is based on; '0' before the first. */
  base: string;
  /** The upload, 0…1. */
  progress: number;
  /** The revision someone else saved when Kaydet found a conflict. */
  conflictActual: string | null;
  /** A newer revision on the server the drawing is not based on. */
  newerRevision: string | null;
  /** The drawing has unsaved changes (what a newer revision says over them; absent: none). */
  dirty?: boolean;
}

export type SaveCellInput = { kind: 'none' } | DatabaseSave | FileSave;

export const SAVE_TEXT: Record<SaveState, (n: number) => string> = {
  saved: () => 'Buluta kaydedildi',
  pending: (n) => `Kaydedilecek: ${n}`,
  saving: () => 'Kaydediliyor…',
  offline_pending: (n) => `Çevrimdışı: ${n} bekliyor`,
  conflict: (n) => `Çakışma: ${n}`,
  error: () => 'Kayıt hatası',
  readonly: () => 'Salt okunur',
  deleted: () => 'Proje silindi',
  revoked: () => 'Erişim kaldırıldı',
  archived: () => 'Proje arşivde',
};

export const LINK_TEXT: Record<LinkState | 'none', string> = { none: '', connecting: 'bağlanıyor', online: 'canlı', reconnecting: 'yeniden bağlanıyor', offline: 'çevrimdışı', auth_required: 'oturum gerekli' };

/** A file project's Kaydet as the cell says it. */
export const FILE_SAVE_TEXT: Record<FileSaveState, (f: FileSave) => string> = {
  saved: (f) => (f.base === '0' ? 'Henüz revizyon yok' : `Buluta kaydedildi · r${f.base}`),
  pending: (f) => (f.base === '0' ? 'Kaydedilmedi' : `Kaydedilmedi · r${f.base} üstüne`),
  encoding: () => 'Dosya hazırlanıyor…',
  uploading: (f) => `Yükleniyor %${Math.round(f.progress * 100)}`,
  verifying: () => 'Sunucu doğruluyor…',
  conflict: (f) => `Çakışma: r${f.conflictActual ?? '?'} kaydedilmiş`,
  // Over unsaved work too: Kaydet cannot write over it (docs/specs/file-revisions.md).
  outdated: (f) => `Yeni revizyon: r${f.newerRevision ?? '?'}${f.dirty ? ' · kaydedilmedi' : ''}`,
  error: () => 'Kayıt hatası',
  readonly: () => 'Salt okunur',
  deleted: () => 'Proje silindi',
  revoked: () => 'Erişim kaldırıldı',
  archived: () => 'Proje arşivde',
};

/** The save cell: hidden without a cloud project; its words and its state (the lamp's colour). */
export function saveCellView(s: SaveCellInput): { hidden: boolean; text: string; state: string | null } {
  if (s.kind === 'none') return { hidden: true, text: '', state: null };
  if (s.kind === 'file') return { hidden: false, text: FILE_SAVE_TEXT[s.state](s), state: s.state };
  return { hidden: false, text: SAVE_TEXT[s.state](s.state === 'conflict' ? s.conflicts : s.pending), state: s.state };
}

/**
 * What a click on the save cell does, as a command, or nothing: a conflict
 * opens the conflicts; a newer revision is offered, never loaded by itself;
 * otherwise it saves, except while read-only or while a Kaydet is on its way.
 * A deleted project or taken access still saves: Kaydet offers a local file.
 */
export function saveCellAction(s: SaveCellInput): string | null {
  if (s.kind === 'none') return null;
  if (s.state === 'conflict') return 'cloud.conflicts';
  if (s.kind === 'file') {
    if (s.state === 'outdated') return 'cloud.openNewest';
    if (s.state === 'readonly' || s.state === 'encoding' || s.state === 'uploading' || s.state === 'verifying') return null;
    return 'file.save';
  }
  return s.state === 'readonly' ? null : 'file.save';
}

/** How long ago, in seconds under a minute, else in minutes; “henüz yok” for never. */
export function ago(at: number | null, now: number): string {
  if (at === null) return 'henüz yok';
  const s = Math.round((now - at) / 1000);
  return s < 60 ? `${s} sn önce` : `${Math.round(s / 60)} dk önce`;
}

/** A database project's cell tip: where it is, how it saves and when it last did, the live link, the error, and whether drafts survive here. */
export function databaseTip(t: {
  state: SaveState;
  /** “<workspace> › <project>”. */
  where: string;
  lastSaved: number | null;
  now: number;
  link: LinkState | 'none';
  error: string;
  /** The browser keeps unsent changes (IndexedDB); the desktop always does. */
  durableDrafts: boolean;
}): { title: string; description: string } {
  const title = 'Bulut kaydı';
  if (t.state === 'deleted')
    return { title, description: `${t.where} sunucuda silindi. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin.` };
  if (t.state === 'revoked')
    return {
      title,
      description: `${t.where} projesine erişiminiz kaldırıldı. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Erişim için proje sahibine başvurun.`,
    };
  if (t.state === 'archived')
    return {
      title,
      description: `${t.where} arşivlenmiş: salt okunurdur, değişiklikler buluta gönderilmiyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Proje sahibi ya da yöneticisi arşivden çıkarınca projeyi yeniden açın.`,
    };
  const lines = [
    `${t.where}. Değişiklikler kendiliğinden kaydedilir; Ctrl+S hemen gönderir.`,
    `Son kayıt: ${ago(t.lastSaved, t.now)}. Canlı bağlantı: ${LINK_TEXT[t.link]}.`,
    t.error,
    t.durableDrafts ? '' : 'Bu tarayıcı taslakları saklayamıyor: kaydedilmeden kapanırsa değişiklikler kaybolur.',
  ];
  return { title, description: lines.filter(Boolean).join(' ') };
}

/** A file project's cell tip: where the drawing stands, and what Kaydet does. */
export function fileTip(t: {
  where: string;
  base: string;
  lastSaved: { revision: string; at: number } | null;
  now: number;
  /** A newer revision on the server: who saved it and when, as far as known (`at` absent: not known). */
  newer: (Omit<NewerRevision, 'at'> & { at?: string | null }) | null;
  error: string;
  link: LinkState | 'none';
  /** The drawing has unsaved changes (kept here as a recovery copy too). */
  dirty: boolean;
}): { title: string; description: string } {
  const link = LINK_TEXT[t.link];
  const lines = [
    `${t.where}, dosya olarak saklanıyor (KCAD revizyonları).`,
    // The number stays apart from its suffix: “revizyon 12'e” would need Turkish vowel harmony by the numeral's reading.
    t.base === '0' ? 'Projenin henüz revizyonu yok.' : `Çizimin dayandığı revizyon: ${t.base}.`,
    'Kendiliğinden kaydedilmez: Kaydet (Ctrl+S) yeni bir revizyon yazar; arada başkası kaydettiyse üzerine yazılmaz.',
    t.lastSaved ? `Bu pencerenin son kaydı: revizyon ${t.lastSaved.revision}, ${ago(t.lastSaved.at, t.now)}.` : '',
    t.newer ? newerTip({ ...t.newer, at: t.newer.at ?? null }, t.dirty) : '',
    t.error,
    link ? `Canlı bağlantı: ${link}.` : '',
    t.dirty ? 'Kaydedilmemiş değişiklikler bu cihazda kurtarma kopyası olarak da saklanıyor.' : '',
  ];
  return { title: 'Bulut kaydı: dosya projesi', description: lines.filter(Boolean).join(' ') };
}

export const SERVER_TEXT: Record<ServerState, string> = { checking: 'Sunucu…', online: 'Sunucu: bağlı', offline: 'Sunucu: yok', incompatible: 'Sunucu: uyumsuz' };

/**
 * The server cell's tip: the service, its build and its contract version
 * when it answers; when its contract is not this app's, the service and
 * build with the reason (which says both contract versions); the reason with
 * what the drawing does without it when it does not answer. `health`: the
 * server's last good answer (`GET /v1/health`), kept while incompatible.
 * `dev`: a development build, which says how to start the server.
 */
export function serverTip(t: { state: ServerState; health: Health | null; detail: string; dev: boolean }): { title: string; description: string } {
  const h = t.health;
  const build = h ? `${h.service} ${h.version}${h.commit ? ` (${h.commit.slice(0, 8)})` : ''}` : '';
  const offline = `${t.detail} Çizim sunucusuz çalışır; kayıt yerel .kcad dosyasına yapılır.${t.dev ? ' Geliştirmede sunucuyu “pnpm api” ile başlatın.' : ''}`;
  const text =
    t.state === 'online'
      ? h
        ? `${build}, sözleşme sürümü ${h.contracts}.`
        : ''
      : t.state === 'incompatible'
        ? `${h ? `${build}. ` : ''}${t.detail}`
        : t.state === 'checking'
          ? 'Sunucuya soruluyor…'
          : offline;
  return { title: 'KentOS sunucusu', description: `${text ? `${text} ` : ''}Hesap, bulut projeleri ve bağlantı denetimi için tıklayın.` };
}

/** A row of the account menu: its header, a separator, or a command with, when it is off for want of a right, why. */
export type AccountRow = { kind: 'header'; label: string } | { kind: 'separator' } | { command: string; detail?: string };

/** The open project's actions and the right each needs. */
export const PROJECT_ACTIONS: readonly { command: string; permission: ProjectPermission }[] = [
  { command: 'cloud.history', permission: 'project.history' },
  { command: 'cloud.share', permission: 'project.share' },
  { command: 'cloud.rename', permission: 'project.edit' },
  { command: 'cloud.delete', permission: 'project.delete' },
];

/**
 * The server cell's menu: who is signed in (and the open project's
 * workspace), signing in or out, the cloud commands, the open project's
 * actions, and the server check. An action on the open project that is off
 * because the account lacks its right says which right, and whom to ask.
 */
export function accountRows(t: {
  /** The signed-in account's name; null when nobody is signed in. */
  user: string | null;
  /** The open cloud project's workspace, when one is open. */
  project: { tenantName: string } | null;
  /** The command is off now. */
  disabled: (command: string) => boolean;
  /** The account has this right in the open project. */
  may: (permission: ProjectPermission) => boolean;
}): AccountRow[] {
  const needs = (a: { command: string; permission: ProjectPermission }): AccountRow =>
    t.project && t.disabled(a.command) && !t.may(a.permission)
      ? { command: a.command, detail: `Bu projede yetkiniz yok (${a.permission}); proje sahibine ya da yöneticisine başvurun.` }
      : { command: a.command };
  return [
    { kind: 'header', label: t.user !== null ? `${t.user}${t.project ? ` · ${t.project.tenantName}` : ''}` : 'Oturum açılmadı' },
    { command: t.user !== null ? 'cloud.signOut' : 'cloud.signIn' },
    { kind: 'separator' },
    { command: 'cloud.open' },
    { command: 'cloud.upload' },
    { command: 'cloud.uploadFile' },
    ...PROJECT_ACTIONS.map(needs),
    { kind: 'separator' },
    { command: 'server.check' },
  ];
}
