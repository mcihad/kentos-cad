import '../../styles/recent.css';
import type { AppContext } from '../../app/context';
import type { RecentFile } from '../../app/recentFiles';
import { h } from '../dom';
import { icon } from '../icons';

/**
 * Rows of the recent files (Son dosyalar), as the start screen and the
 * application menu list them: the file's name, what it held and when, a
 * click opens it again, × forgets it.
 */

/** When, in words: az önce, 5 dk önce, 3 sa önce, then the date. */
export function agoText(ms: number): string {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 60) return 'az önce';
  if (s < 3600) return `${Math.round(s / 60)} dk önce`;
  if (s < 86400) return `${Math.round(s / 3600)} sa önce`;
  return new Date(ms).toLocaleDateString('tr-TR', { day: 'numeric', month: 'short', year: 'numeric' });
}

/** One recent file; `before` runs ahead of opening it (a menu closes). */
export function recentFileRow(ctx: AppContext, f: RecentFile, before?: () => void): HTMLElement {
  const cls = 'recent';
  const open = h(
    'button',
    { class: `${cls}__open`, type: 'button', dataset: { recent: f.id }, title: f.name },
    h('span', { class: `${cls}__icon` }, icon('fileOpen', 16)),
    h('span', { class: `${cls}__text` }, h('span', { class: `${cls}__name` }, f.name.replace(/\.kcad$/i, '')), h('span', { class: `${cls}__meta` }, `${f.info} · ${agoText(f.at)}`)),
  );
  open.addEventListener('click', () => {
    before?.();
    void ctx.files.openRecent(f);
  });
  const forget = h('button', { class: `${cls}__forget`, type: 'button', 'aria-label': `“${f.name}” dosyasını listeden kaldır`, title: 'Listeden kaldır' }, icon('close', 12));
  forget.addEventListener('click', () => void ctx.files.recent.remove(f.id));
  return h('div', { class: cls, role: 'listitem' }, open, forget);
}
