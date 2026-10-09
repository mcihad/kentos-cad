import type { AppContext } from '../../app/context';
import { mergeLayers, mergeSummary } from '../../app/layerActions';
import { h, replaceChildren } from '../dom';
import { field, summaryLine } from '../io/common';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const MERGE_TITLE = 'Katmanları birleştir';

/**
 * Katmanları birleştir (docs/adr/0177 §3; the desktop's `layer_merge.rs`): the drawing's layers with their paths and
 * object counts, each with its box, and the target in a list (the one the Katmanlar panel's menu named, else the active
 * layer). A locked layer and the target cannot be checked. Birleştir writes, in one step “Katmanları birleştir”, and
 * closes; what keeps it from writing is said above the list.
 */
export function openMergeLayers(ctx: AppContext, target?: string): void {
  const { doc } = ctx;
  const layers = doc.layers;
  const all = layers.leaves();
  let into = target && layers.get(target)?.type === 'layer' ? target : layers.active.value;
  const checked = new Set<string>();
  const counts = doc.countByLayer();

  const select = h(
    'select',
    { class: 'field', 'aria-label': 'Hedef katman' },
    ...all.map((l) => h('option', { value: l.id, selected: l.id === into }, layers.path(l.id))),
  ) as HTMLSelectElement;
  const summary = h('div', { class: 'io-summary' });
  const tbody = h('tbody');
  const table = h(
    'div',
    { class: 'io-table-wrap merge-rows' },
    h('table', { class: 'io-table' }, h('thead', null, h('tr', null, h('th', { class: 'find-check' }), h('th', null, 'Katman'), h('th', { class: 'num' }, 'Nesne'))), tbody),
  );
  const merge = h('button', { class: 'btn btn--primary', type: 'button' }, 'Birleştir');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');

  function rowOf(id: string): HTMLElement {
    const locked = layers.isLocked(id);
    const isTarget = id === into;
    const box = h('input', { type: 'checkbox', checked: checked.has(id), disabled: locked || isTarget, 'aria-label': layers.path(id) });
    box.addEventListener('change', () => {
      if (box.checked) checked.add(id);
      else checked.delete(id);
      refresh();
    });
    const why = locked ? 'Kilitli: birleştirilmez.' : isTarget ? 'Hedef katman.' : undefined;
    return h(
      'tr',
      { class: `merge-row${locked || isTarget ? ' find-row--blocked' : ''}`, ...(why && { title: why }) },
      h('td', { class: 'find-check' }, box),
      h('td', { class: 'io-table__name' }, locked ? icon('lock', 12) : null, layers.path(id), isTarget ? h('span', { class: 'merge-row__target' }, ' (hedef)') : null),
      h('td', { class: 'num' }, String(counts.get(id) ?? 0)),
    );
  }

  function refresh(): void {
    checked.delete(into);
    const said = mergeSummary(ctx, [...checked], into);
    replaceChildren(summary, ...said.lines.map(([kind, text]) => summaryLine(kind, text)));
    replaceChildren(tbody, ...all.map((l) => rowOf(l.id)));
    merge.disabled = !said.ready;
  }

  const dialog = new Dialog({
    title: MERGE_TITLE,
    width: 560,
    className: 'dialog--io dialog--merge',
    content: [h('div', { class: 'io-row' }, field('Hedef katman', select, undefined, 'grow')), summary, table],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, merge],
  });
  select.addEventListener('change', () => {
    into = select.value;
    refresh();
  });
  merge.addEventListener('click', () => {
    if (mergeLayers(ctx, [...checked], into)) dialog.close();
  });
  cancel.addEventListener('click', () => dialog.close());
  refresh();
}
