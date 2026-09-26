import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import type { FileProjectSave, FileSaveState } from '../../app/cloud/fileProject';
import type { SaveState } from '../../app/cloud/sync';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { DisposableStore } from '../../core/disposable';
import { h } from '../dom';
import { PopupMenu } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';

/**
 * Status bar cells of the cloud (CLAUDE.md §21.1): the save state of an open
 * cloud project, with what is waiting, and the account menu behind the
 * server cell. "Buluta kaydedildi" is shown only after the server's answer.
 * A file project (docs/adr/0038, TODOS.md SYNC-04) shows its Kaydet's
 * stages apart: the drawing being written, the upload with how far, the
 * server checking and committing, and the revision once it is saved.
 */

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

export const LINK_TEXT = { none: '', connecting: 'bağlanıyor', online: 'canlı', reconnecting: 'yeniden bağlanıyor', offline: 'çevrimdışı', auth_required: 'oturum gerekli' } as const;

/** A file project's Kaydet as the cell says it. */
export const FILE_SAVE_TEXT: Record<FileSaveState, (f: FileProjectSave) => string> = {
  saved: (f) => (f.base.value === '0' ? 'Henüz revizyon yok' : `Buluta kaydedildi · r${f.base.value}`),
  pending: (f) => (f.base.value === '0' ? 'Kaydedilmedi' : `Kaydedilmedi · r${f.base.value} üstüne`),
  encoding: () => 'Dosya hazırlanıyor…',
  uploading: (f) => `Yükleniyor %${Math.round(f.progress.value * 100)}`,
  verifying: () => 'Sunucu doğruluyor…',
  conflict: (f) => `Çakışma: r${f.conflict.value?.actual ?? '?'} kaydedilmiş`,
  outdated: (f) => `Yeni revizyon: r${f.newer.value?.revision ?? '?'}`,
  error: () => 'Kayıt hatası',
  readonly: () => 'Salt okunur',
  deleted: () => 'Proje silindi',
  revoked: () => 'Erişim kaldırıldı',
  archived: () => 'Proje arşivde',
};

/** The open file project's Kaydet state in words (the app menu uses it too). */
export const fileSaveText = (f: FileProjectSave): string => FILE_SAVE_TEXT[f.state.value](f);

function ago(ms: number | null): string {
  if (ms === null) return 'henüz yok';
  const s = Math.round((Date.now() - ms) / 1000);
  return s < 60 ? `${s} sn önce` : `${Math.round(s / 60)} dk önce`;
}

/** The save cell: hidden without a cloud project; a click does the next useful thing. */
export function saveCell(ctx: AppContext, d: DisposableStore): HTMLElement {
  const text = h('span');
  const cell = h('button', { class: 'status__cell status__btn status__save', type: 'button', hidden: true }, h('span', { class: 'status__lamp', 'aria-hidden': 'true' }), text);
  // Subscriptions to the current project's sync; replaced when another project opens.
  let per: (() => void)[] = [];
  const drop = () => {
    for (const u of per) u();
    per = [];
  };
  d.add(drop);
  const render = () => {
    const sync = ctx.cloud.sync.value;
    const file = ctx.cloud.file.value;
    cell.hidden = !sync && !file;
    if (file) {
      text.textContent = fileSaveText(file);
      cell.dataset.state = file.state.value;
      return;
    }
    if (!sync) return;
    const state = sync.state.value;
    const count = state === 'conflict' ? sync.conflicts.value.length : sync.pending.value;
    text.textContent = SAVE_TEXT[state](count);
    cell.dataset.state = state;
  };
  d.add(
    ctx.cloud.sync.subscribe((sync) => {
      drop();
      if (sync) per = [sync.state.subscribe(render), sync.pending.subscribe(render), sync.conflicts.subscribe(render)];
      render();
    }, true),
  );
  // A database project has an autosave, a file project a Kaydet; never both at once.
  d.add(
    ctx.cloud.file.subscribe((file) => {
      drop();
      if (file) per = [file.state.subscribe(render), file.progress.subscribe(render), file.base.subscribe(render), file.newer.subscribe(render)];
      render();
    }, true),
  );
  cell.addEventListener('click', () => {
    const file = ctx.cloud.file.value;
    if (file) {
      const s = file.state.value;
      if (s === 'conflict') ctx.commands.execute('cloud.conflicts');
      // Someone saved a newer revision: offered, never loaded by itself.
      else if (s === 'outdated') ctx.commands.execute('cloud.openNewest');
      else if (s !== 'readonly' && s !== 'encoding' && s !== 'uploading' && s !== 'verifying') ctx.commands.execute('file.save');
      return;
    }
    const state = ctx.cloud.sync.value?.state.value;
    if (state === 'conflict') ctx.commands.execute('cloud.conflicts');
    // Deleted, or the access taken away: Kaydet offers a local file, the one place the drawing can still go.
    else if (state !== 'readonly') ctx.commands.execute('file.save');
  });
  d.add(
    tooltip(
      cell,
      () => {
        const sync = ctx.cloud.sync.value;
        const file = ctx.cloud.file.value;
        const p = ctx.cloud.project.value;
        if (file && p) return { title: 'Bulut kaydı: dosya projesi', description: fileTip(ctx, file, `${p.tenantName} › ${p.name}`) };
        if (!sync || !p) return null;
        const link = LINK_TEXT[ctx.cloud.link.value];
        if (sync.state.value === 'deleted')
          return {
            title: 'Bulut kaydı',
            description: `${p.tenantName} › ${p.name} sunucuda silindi. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin.`,
          };
        if (sync.state.value === 'revoked')
          return {
            title: 'Bulut kaydı',
            description: `${p.tenantName} › ${p.name} projesine erişiminiz kaldırıldı. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Erişim için proje sahibine başvurun.`,
          };
        if (sync.state.value === 'archived')
          return {
            title: 'Bulut kaydı',
            description: `${p.tenantName} › ${p.name} arşivlenmiş: salt okunurdur, değişiklikler buluta gönderilmiyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Proje sahibi ya da yöneticisi arşivden çıkarınca projeyi yeniden açın.`,
          };
        const lines = [
          `${p.tenantName} › ${p.name}. Değişiklikler kendiliğinden kaydedilir; Ctrl+S hemen gönderir.`,
          `Son kayıt: ${ago(sync.lastSaved.value)}. Canlı bağlantı: ${link}.`,
          sync.error.value,
          ctx.cloud.durableDrafts ? '' : 'Bu tarayıcı taslakları saklayamıyor: kaydedilmeden kapanırsa değişiklikler kaybolur.',
        ].filter(Boolean);
        return { title: 'Bulut kaydı', description: lines.join(' ') };
      },
      'top',
    ),
  );
  return cell;
}

/** What a file project's cell says on hover: where the drawing stands, and what Kaydet does. */
function fileTip(ctx: AppContext, f: FileProjectSave, where: string): string {
  const last = f.lastSaved.value;
  const link = LINK_TEXT[ctx.cloud.link.value];
  const lines = [
    `${where}, dosya olarak saklanıyor (KCAD revizyonları).`,
    f.base.value === '0' ? 'Projenin henüz revizyonu yok.' : `Çizim revizyon ${f.base.value}'e dayanıyor.`,
    'Kendiliğinden kaydedilmez: Kaydet (Ctrl+S) yeni bir revizyon yazar; arada başkası kaydettiyse üzerine yazılmaz.',
    last ? `Bu pencerenin son kaydı: revizyon ${last.revision}, ${ago(last.at)}.` : '',
    f.newer.value ? `Sunucuda daha yeni revizyon var: ${f.newer.value.revision}${f.newer.value.by ? ` (${f.newer.value.by})` : ''}; açmak için tıklayın.` : '',
    f.error.value,
    link ? `Canlı bağlantı: ${link}.` : '',
    ctx.doc.dirty.value ? 'Kaydedilmemiş değişiklikler bu cihazda kurtarma kopyası olarak da saklanıyor.' : '',
  ].filter(Boolean);
  return lines.join(' ');
}

/** The server cell's menu: the account and the cloud commands. */
export function accountMenu(ctx: AppContext, anchor: HTMLElement): void {
  const me = ctx.cloud.me.value;
  const p = ctx.cloud.project.value;
  // An action on the open project that this account may not take says which right it lacks.
  const needs = (id: string, permission: ProjectPermission) => {
    const item = commandItem(ctx, id);
    if (p && item.disabled && !ctx.cloud.may(permission)) item.detail = `Bu projede yetkiniz yok (${permission}); proje sahibine ya da yöneticisine başvurun.`;
    return item;
  };
  PopupMenu.open(
    [
      { kind: 'header', label: me ? `${me.user.displayName}${p ? ` · ${p.tenantName}` : ''}` : 'Oturum açılmadı' },
      commandItem(ctx, me ? 'cloud.signOut' : 'cloud.signIn'),
      { kind: 'separator' },
      commandItem(ctx, 'cloud.open'),
      commandItem(ctx, 'cloud.upload'),
      commandItem(ctx, 'cloud.uploadFile'),
      needs('cloud.history', 'project.history'),
      needs('cloud.share', 'project.share'),
      needs('cloud.rename', 'project.edit'),
      needs('cloud.delete', 'project.delete'),
      { kind: 'separator' },
      commandItem(ctx, 'server.check'),
    ],
    anchor.getBoundingClientRect(),
    { placement: 'below', owner: anchor },
  );
}

