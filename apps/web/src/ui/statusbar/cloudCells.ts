import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import type { SaveState } from '../../app/cloud/sync';
import type { DisposableStore } from '../../core/disposable';
import { h } from '../dom';
import { PopupMenu } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';

/**
 * Status bar cells of the cloud (CLAUDE.md §21.1): the save state of an open
 * cloud project, with what is waiting, and the account menu behind the
 * server cell. "Buluta kaydedildi" is shown only after the server's answer.
 */

const SAVE_TEXT: Record<SaveState, (n: number) => string> = {
  saved: () => 'Buluta kaydedildi',
  pending: (n) => `Kaydedilecek: ${n}`,
  saving: () => 'Kaydediliyor…',
  offline_pending: (n) => `Çevrimdışı: ${n} bekliyor`,
  conflict: (n) => `Çakışma: ${n}`,
  error: () => 'Kayıt hatası',
  readonly: () => 'Salt okunur',
  deleted: () => 'Proje silindi',
};

const LINK_TEXT = { none: '', connecting: 'bağlanıyor', online: 'canlı', reconnecting: 'yeniden bağlanıyor', offline: 'çevrimdışı', auth_required: 'oturum gerekli' } as const;

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
    cell.hidden = !sync;
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
  cell.addEventListener('click', () => {
    const state = ctx.cloud.sync.value?.state.value;
    if (state === 'conflict') ctx.commands.execute('cloud.conflicts');
    // Deleted: Kaydet offers a local file, the one place the drawing can still go.
    else if (state !== 'readonly') ctx.commands.execute('file.save');
  });
  d.add(
    tooltip(
      cell,
      () => {
        const sync = ctx.cloud.sync.value;
        const p = ctx.cloud.project.value;
        if (!sync || !p) return null;
        const link = LINK_TEXT[ctx.cloud.link.value];
        if (sync.state.value === 'deleted')
          return {
            title: 'Bulut kaydı',
            description: `${p.tenantName} › ${p.name} sunucuda silindi. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin.`,
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

/** The server cell's menu: the account and the cloud commands. */
export function accountMenu(ctx: AppContext, anchor: HTMLElement): void {
  const me = ctx.cloud.me.value;
  const p = ctx.cloud.project.value;
  // An action on the open project that this account may not take says which right it lacks.
  const needs = (id: string, capability: string) => {
    const item = commandItem(ctx, id);
    if (p && item.disabled && !ctx.cloud.can(p.tenantId, capability)) item.detail = `Yetkiniz yok (${capability}); kurum yöneticinize başvurun.`;
    return item;
  };
  PopupMenu.open(
    [
      { kind: 'header', label: me ? `${me.user.displayName}${p ? ` · ${p.tenantName}` : ''}` : 'Oturum açılmadı' },
      commandItem(ctx, me ? 'cloud.signOut' : 'cloud.signIn'),
      { kind: 'separator' },
      commandItem(ctx, 'cloud.open'),
      commandItem(ctx, 'cloud.upload'),
      needs('cloud.rename', 'project.edit'),
      needs('cloud.delete', 'project.delete'),
      { kind: 'separator' },
      commandItem(ctx, 'server.check'),
    ],
    anchor.getBoundingClientRect(),
    { placement: 'below', owner: anchor },
  );
}

