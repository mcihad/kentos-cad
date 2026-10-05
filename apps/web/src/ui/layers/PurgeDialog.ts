import type { AppContext } from '../../app/context';
import { purgeFoundNow, purgeUnused } from '../../app/layerPurge';
import { PURGE_KINDS, type PurgeKind } from '../../model/layerPurge';
import { h, replaceChildren } from '../dom';
import { summaryLine } from '../io/common';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const PURGE_TITLE = 'Kullanılmayanları temizle';

/** Each kind's heading in the list. */
export const PURGE_HEADINGS: Record<PurgeKind, string> = {
  layers: 'Boş katmanlar',
  groups: 'Boş kalan gruplar',
  blocks: 'Kullanılmayan bloklar',
  symbols: 'Projenin kullanılmayan sembolleri',
  assets: 'Projenin kullanılmayan varlıkları',
};

/**
 * Kullanılmayanları temizle (docs/adr/0177 §5; the desktop's `layer_purge.rs`): what nothing uses, by kind, each with its
 * box, checked; a locked empty layer is listed to be unlocked first and cannot be checked. Temizle removes the checked
 * ones nothing staying uses and closes.
 */
export function openPurge(ctx: AppContext): void {
  const found = purgeFoundNow(ctx);
  const checked = new Set(PURGE_KINDS.flatMap((k) => found[k].filter((e) => !('locked' in e && e.locked)).map((e) => `${k}:${e.id}`)));
  const any = PURGE_KINDS.some((k) => found[k].length);
  const summary = h('div', { class: 'io-summary' });
  const list = h('div', { class: 'io-table-wrap purge-rows' });
  const purge = h('button', { class: 'btn btn--primary', type: 'button' }, 'Temizle') as HTMLButtonElement;
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç') as HTMLButtonElement;

  function rowOf(kind: PurgeKind, id: string, words: string, locked: boolean): HTMLElement {
    const key = `${kind}:${id}`;
    const box = h('input', { type: 'checkbox', checked: checked.has(key), disabled: locked, 'aria-label': words }) as HTMLInputElement;
    box.addEventListener('change', () => {
      if (box.checked) checked.add(key);
      else checked.delete(key);
      refresh();
    });
    return h(
      'label',
      { class: `purge-row${locked ? ' purge-row--locked' : ''}`, ...(locked && { title: 'Kilitli: silmek için önce kilidini Katmanlar panelinden açın.' }) },
      box,
      locked ? icon('lock', 12) : null,
      h('span', { class: 'purge-row__words' }, words),
      locked ? h('span', { class: 'purge-row__note' }, 'kilitli') : null,
    );
  }

  function refresh(): void {
    const lines = any
      ? [
          summaryLine('info', 'İşaretlenenler silinir. Katmanlar, gruplar ve bloklar tek adımda geri alınır; projenin kitaplığından silinen semboller ve varlıklar geri alınamaz.'),
          summaryLine(checked.size ? 'ok' : 'info', checked.size ? `${checked.size} öğe işaretli.` : 'Silinecekleri işaretleyin.'),
        ]
      : [summaryLine('ok', 'Temizlenecek bir şey yok: her katmanda nesne var; bloklar, projenin sembolleri ve varlıkları kullanılıyor.')];
    replaceChildren(summary, ...lines);
    purge.disabled = !checked.size;
  }

  replaceChildren(
    list,
    ...PURGE_KINDS.filter((k) => found[k].length).flatMap((k) => [
      h('div', { class: 'purge-heading' }, `${PURGE_HEADINGS[k]} (${found[k].length})`),
      ...found[k].map((e) => rowOf(k, e.id, 'path' in e ? e.path : e.name, 'locked' in e && e.locked)),
    ]),
  );
  list.hidden = !any;

  const dialog = new Dialog({
    title: PURGE_TITLE,
    width: 560,
    className: 'dialog--io dialog--purge',
    content: [summary, list],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, purge],
  });
  purge.addEventListener('click', () => {
    const ids = (k: PurgeKind) => [...checked].filter((x) => x.startsWith(`${k}:`)).map((x) => x.slice(k.length + 1));
    if (purgeUnused(ctx, Object.fromEntries(PURGE_KINDS.map((k) => [k, ids(k)])) as Record<PurgeKind, string[]>)) dialog.close();
  });
  cancel.addEventListener('click', () => dialog.close());
  refresh();
}
