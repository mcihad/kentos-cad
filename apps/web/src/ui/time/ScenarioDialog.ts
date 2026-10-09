import '../../styles/time.css';
import type { AppContext } from '../../app/context';
import { createScenario } from '../../app/scenarios';
import { inScenario, SCENARIO_NAME_MAX, SCENARIO_NOTE_MAX } from '../../model/temporalRules';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { field, summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const SCENARIO_TITLE = 'Senaryo oluştur';

/**
 * Senaryo oluştur (docs/adr/0210 §9, §10; the desktop's `scenario_create.rs`): Ad, Not, the base layers to copy (the
 * active one checked; none, an empty scenario), Nesneleri kopyala. A locked layer's objects are not copied: it can be
 * checked only with Nesneleri kopyala off. Oluştur writes through `cad.scenarios.edit` (one step “Senaryo oluştur”),
 * then the scenario is shown.
 */
export function openScenarioCreate(ctx: AppContext): void {
  const { doc } = ctx;
  const layers = doc.layers;
  const bases = layers.leaves().filter((l) => !l.service && !inScenario(layers, l.id));
  const checked = new Set<string>(bases.some((l) => l.id === layers.active.value) ? [layers.active.value] : []);
  const counts = doc.countByLayer();
  let n = 1;
  const taken = new Set(layers.all().map((x) => x.name));
  while (taken.has(`Senaryo ${n}`)) n++;

  const name = h('input', { class: 'field', 'aria-label': 'Ad', value: `Senaryo ${n}`, maxlength: String(SCENARIO_NAME_MAX), spellcheck: 'false' }) as HTMLInputElement;
  const note = h('textarea', { class: 'field scenario-note', 'aria-label': 'Not', rows: '2', maxlength: String(SCENARIO_NOTE_MAX), placeholder: 'Önerinin kısa açıklaması (isteğe bağlı)' }) as HTMLTextAreaElement;
  const copy = h('input', { type: 'checkbox', checked: true }) as HTMLInputElement;
  const summary = h('div', { class: 'io-summary' });
  const tbody = h('tbody');
  const table = h(
    'div',
    { class: 'io-table-wrap scenario-rows' },
    h('table', { class: 'io-table' }, h('thead', null, h('tr', null, h('th', { class: 'find-check' }), h('th', null, 'Ana katman'), h('th', { class: 'num' }, 'Nesne'))), tbody),
  );
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const create = h('button', { class: 'btn btn--primary', type: 'button' }, 'Oluştur');

  function rowOf(id: string): HTMLElement {
    const locked = layers.isLocked(id) && copy.checked;
    const box = h('input', { type: 'checkbox', checked: checked.has(id), disabled: locked, 'aria-label': layers.path(id) }) as HTMLInputElement;
    box.addEventListener('change', () => {
      if (box.checked) checked.add(id);
      else checked.delete(id);
      refresh();
    });
    return h(
      'tr',
      { class: `scenario-row${locked ? ' find-row--blocked' : ''}`, ...(locked && { title: 'Kilitli: nesneleri kopyalanmaz. Nesneleri kopyala kapalıyken seçilebilir.' }) },
      h('td', { class: 'find-check' }, box),
      h('td', { class: 'io-table__name' }, layers.isLocked(id) ? icon('lock', 12) : null, layers.path(id)),
      h('td', { class: 'num' }, String(counts.get(id) ?? 0)),
    );
  }

  function refresh(): void {
    // A locked layer checked while copying is left out (its objects would not be copied).
    if (copy.checked) for (const id of checked) if (layers.isLocked(id)) checked.delete(id);
    replaceChildren(tbody, ...bases.map((l) => rowOf(l.id)));
    const objects = copy.checked ? [...checked].reduce((s, id) => s + (counts.get(id) ?? 0), 0) : 0;
    const words = name.value.trim();
    create.disabled = !words;
    replaceChildren(
      summary,
      !words
        ? summaryLine('warn', 'Senaryonun adını yazın.')
        : checked.size
          ? summaryLine('ok', `${checked.size} ana katman kopyalanır${copy.checked ? `, ${objects} nesneyle` : ', nesnesiz'}; senaryo ağacın en üstüne gelir ve gösterilir.`)
          : summaryLine('info', 'Katman seçilmedi: boş bir senaryo oluşur; önerinin katmanlarını sonra ekleyebilirsiniz.'),
    );
  }

  const dialog = new Dialog({
    title: SCENARIO_TITLE,
    width: 600,
    className: 'dialog--io dialog--scenario',
    content: [
      h('div', { class: 'io-row' }, field('Ad', name, undefined, 'grow')),
      h('div', { class: 'io-row' }, field('Not', note, undefined, 'grow')),
      h('div', { class: 'io-row' }, field('Kopyalanacak ana katmanlar', table, undefined, 'grow')),
      h('div', { class: 'io-row' }, field('Nesneler', h('label', { class: 'io-check' }, copy, 'Nesneleri kopyala'))),
      summary,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, create],
  });
  name.addEventListener('input', refresh);
  copy.addEventListener('change', refresh);
  cancel.addEventListener('click', () => dialog.close());
  create.addEventListener('click', () => {
    const text = note.value.trim();
    const id = createScenario(ctx, { name: name.value, layers: bases.filter((l) => checked.has(l.id)).map((l) => l.id), copyObjects: copy.checked, ...(text && { note: text }) });
    if (id) dialog.close();
  });
  refresh();
  queueMicrotask(() => name.select());
}
