import type { AppContext } from '../../app/context';
import { DisposableStore, listen } from '../../core/disposable';
import type { TopologyRule } from '../../contracts/generated/TopologyRule';
import type { TopologyRuleKind } from '../../contracts/generated/TopologyRuleKind';
import type { TopologySettings } from '../../contracts/generated/TopologySettings';
import { nextRuleId, TOPOLOGY_KINDS, TOPOLOGY_TOLERANCE, toleranceHolds, toleranceOf, valueHolds } from '../../model/topologyRules';
import { topologyNames } from '../bottom/topologyRun';
import { h, replaceChildren } from '../dom';
import { field as labelled, summaryLine } from '../io/common';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { askUnsaved } from '../widgets/confirm';

/** The window's title, which a trace names it by. */
export const RULES_TITLE = 'Topoloji kuralları';

/**
 * Topoloji kuralları (docs/adr/0202 §6; the desktop's `topology/rules.rs`): the tolerance and the rules in a table,
 * each its layer, kind, other layer and value, and how many exceptions it has. Kural ekle puts a rule on the active
 * layer; Sil takes the chosen one, its exceptions with it. Lengths are typed in the project's unit, angles in its angle
 * unit. The first problem is said under the table and Kaydet waits for it. Kaydet writes the project's setting (not an
 * undo step, as the layer states).
 */

/** A rule as the window edits it: its value as typed (empty: the kind's default). */
export interface RuleRow {
  id: string;
  kind: TopologyRuleKind;
  layer: string;
  other: string;
  value: string;
}

/** The project's unit for a kind's value: metres in the drawing's unit, radians in the angle unit. */
export interface RuleUnits {
  lengthToMetres(v: number): number;
  lengthFromMetres(m: number): number;
  /** How many radians one of the angle unit is. */
  angleRadians: number;
}

const decimal = (text: string): number => (/^\s*[-+]?(\d+([.,]\d*)?|[.,]\d+)\s*$/.test(text) ? Number(text.trim().replace(',', '.')) : Number.NaN);

/** Round a shown value to what the window types (no gürültü from the unit's conversion). */
const shown = (v: number): string => String(Number(v.toPrecision(12)));

/** A rule as the window shows it. */
export function rowOf(r: TopologyRule, u: RuleUnits): RuleRow {
  const kind = TOPOLOGY_KINDS[r.kind];
  const value = r.value === undefined || !kind.value ? '' : kind.value === 'length' ? shown(u.lengthFromMetres(r.value)) : shown(r.value / u.angleRadians);
  return { id: r.id, kind: r.kind, layer: r.layer, other: r.other ?? '', value };
}

/** The rule a row writes: another layer only between layers, a value only where the kind takes one. */
export function ruleOf(r: RuleRow, u: RuleUnits): TopologyRule {
  const kind = TOPOLOGY_KINDS[r.kind];
  const out: TopologyRule = { id: r.id, kind: r.kind, layer: r.layer };
  if (kind.between && r.other) out.other = r.other;
  const v = decimal(r.value);
  if (kind.value && r.value.trim() && Number.isFinite(v)) out.value = kind.value === 'length' ? u.lengthToMetres(v) : v * u.angleRadians;
  return out;
}

/** What is wrong with the window's values, in the order a reader meets them; null when they hold. */
export function rulesProblem(tolerance: string, rows: readonly RuleRow[], u: RuleUnits, layerName: (id: string) => string | null): string | null {
  const t = decimal(tolerance);
  if (!Number.isFinite(t) || !toleranceHolds(u.lengthToMetres(t))) return 'Tolerans 0,000001 m ile 1 m arasında bir sayı olmalı.';
  for (const [i, r] of rows.entries()) {
    const n = `${i + 1}. kural`;
    const kind = TOPOLOGY_KINDS[r.kind];
    if (!layerName(r.layer)) return `${n}: katman seçin.`;
    if (kind.between && !layerName(r.other)) return `${n}: öbür katmanı seçin.`;
    if (kind.between && r.other === r.layer) return `${n}: öbür katman kuralın kendi katmanı olamaz.`;
    if (kind.value && r.value.trim()) {
      const v = decimal(r.value);
      const si = kind.value === 'length' ? u.lengthToMetres(v) : v * u.angleRadians;
      if (!Number.isFinite(v) || !valueHolds(kind.value, si)) return kind.value === 'length' ? `${n}: değer sıfırdan büyük bir uzunluk olmalı.` : `${n}: açı sıfırdan büyük, dik açıdan küçük olmalı.`;
    }
  }
  return null;
}

/** Opens Topoloji kuralları. */
export function openTopologyRules(ctx: AppContext): void {
  const { doc, format } = ctx;
  const d = new DisposableStore();
  const units: RuleUnits = {
    lengthToMetres: (v) => format.toMetres(v),
    lengthFromMetres: (m) => format.fromMetres(m),
    angleRadians: doc.settings.angleUnit.value === 'grad' ? Math.PI / 200 : Math.PI / 180,
  };
  const before = doc.settings.topology.value;
  const savedTolerance = shown(format.fromMetres(toleranceOf(before)));
  const saved = (before?.rules ?? []).map((r) => rowOf(r, units));
  let tolerance = savedTolerance;
  let rows: RuleRow[] = saved.map((r) => ({ ...r }));
  let chosen = rows.length ? 0 : -1;
  const kinds = topologyNames().kinds;
  const layers = () => doc.layers.leaves();
  const layerName = (id: string) => (id ? (doc.layers.get(id)?.type === 'layer' ? doc.layers.path(id) : null) : null);
  const exceptionsOf = (id: string) => (before?.exceptions ?? []).filter((x) => x.rule === id).length;

  const body = h('tbody');
  const summary = h('div', { class: 'io-summary fields-summary' });
  const button = (words: string, glyph: string | null, primary = false) =>
    h('button', { class: primary ? 'btn btn--primary' : 'btn', type: 'button' }, glyph ? icon(glyph, 14) : null, words) as HTMLButtonElement;
  const add = button('Kural ekle', 'plus');
  const remove = button('Sil', 'erase');
  const save = button('Kaydet', null, true);
  const cancel = button('Vazgeç', null);
  const toleranceInput = h('input', { class: 'field num topo-rules__tolerance', type: 'text', value: tolerance, 'aria-label': 'Tolerans', spellcheck: 'false' }) as HTMLInputElement;
  toleranceInput.addEventListener('input', () => {
    tolerance = toleranceInput.value;
    check();
  });

  const settingsOf = (): TopologySettings | null => {
    const rules = rows.map((r) => ruleOf(r, units));
    const t = units.lengthToMetres(decimal(tolerance));
    const exceptions = (before?.exceptions ?? []).filter((x) => rules.some((r) => r.id === x.rule));
    const out: TopologySettings = {};
    if (Math.abs(t - TOPOLOGY_TOLERANCE) > 1e-12) out.tolerance = t;
    if (rules.length) out.rules = rules;
    if (exceptions.length) out.exceptions = exceptions;
    return Object.keys(out).length ? out : null;
  };
  const changed = () => JSON.stringify(settingsOf()) !== JSON.stringify(before ?? null);
  /** Set as Kaydet or Vazgeç closes the window: no question then. */
  let done = false;
  const problem = () => rulesProblem(tolerance, rows, units, layerName);

  const select = (label: string, value: string, options: { value: string; text: string }[], disabled: boolean, set: (v: string) => void) => {
    const el = h('select', { class: 'field fields-cell', 'aria-label': label, disabled }, options.map((o) => h('option', { value: o.value, selected: o.value === value }, o.text))) as HTMLSelectElement;
    el.addEventListener('change', () => set(el.value));
    return el;
  };

  function rowEl(r: RuleRow, i: number): HTMLElement {
    const kind = TOPOLOGY_KINDS[r.kind];
    const meta = kinds.find((k) => k.key === r.kind);
    const layerOptions = [{ value: '', text: '—' }, ...layers().map((l) => ({ value: l.id, text: doc.layers.path(l.id) }))];
    const unit = kind.value === 'length' ? format.lengthUnitLabel : kind.value === 'angle' ? format.angleUnitLabel : '';
    const placeholder = meta?.defaultValue !== undefined && kind.value ? (kind.value === 'length' ? shown(units.lengthFromMetres(meta.defaultValue)) : shown(meta.defaultValue / units.angleRadians)) : '';
    const value = h('input', { class: 'field fields-cell num', type: 'text', value: r.value, 'aria-label': meta?.valueLabel ?? 'Değer', placeholder, spellcheck: 'false', disabled: !kind.value }) as HTMLInputElement;
    value.addEventListener('input', () => {
      r.value = value.value;
      check();
    });
    const tr = h(
      'tr',
      { class: i === chosen ? 'is-selected' : null, 'aria-selected': String(i === chosen) },
      h('td', null, select('Katman', r.layer, layerOptions, false, (v) => ((r.layer = v), render()))),
      h(
        'td',
        null,
        select(
          'Kural',
          r.kind,
          kinds.map((k) => ({ value: k.key, text: k.label })),
          false,
          (v) => {
            r.kind = v as TopologyRuleKind;
            if (!TOPOLOGY_KINDS[r.kind].between) r.other = '';
            if (!TOPOLOGY_KINDS[r.kind].value) r.value = '';
            render();
          },
        ),
      ),
      h('td', null, select('Öbür katman', kind.between ? r.other : '', layerOptions, !kind.between, (v) => ((r.other = v), render()))),
      h('td', null, h('div', { class: 'topo-rules__value' }, value, h('span', { class: 'topo-rules__unit' }, unit))),
      h('td', { class: 'num' }, String(exceptionsOf(r.id))),
    );
    tr.addEventListener('mousedown', () => {
      if (chosen !== i) {
        chosen = i;
        for (const [k, row] of [...body.children].entries()) row.classList.toggle('is-selected', k === i);
        remove.disabled = chosen < 0;
      }
    });
    return tr;
  }

  function check(): void {
    const p = problem();
    const n = rows.length;
    replaceChildren(summary, p ? summaryLine('error', p) : summaryLine('ok', n ? `${n} kural; denetim alt panelin Topoloji sekmesinde.` : 'Kural yok: topoloji denetlenmez.'));
    save.disabled = p !== null || !changed();
  }

  function render(): void {
    replaceChildren(body, ...rows.map((r, i) => rowEl(r, i)));
    remove.disabled = chosen < 0;
    check();
  }

  function write(): boolean {
    doc.settings.assign({ topology: settingsOf() });
    ctx.log.info(`Topoloji kuralları kaydedildi: ${rows.length} kural.`);
    return true;
  }

  add.addEventListener('click', () => {
    const active = doc.layers.active.value;
    const layer = layerName(active) ? active : (layers()[0]?.id ?? '');
    rows.push({ id: nextRuleId(rows.map((r) => ruleOf(r, units))), kind: 'mustNotOverlap', layer, other: '', value: '' });
    chosen = rows.length - 1;
    render();
  });
  remove.addEventListener('click', () => {
    if (chosen < 0) return;
    rows.splice(chosen, 1);
    chosen = Math.min(chosen, rows.length - 1);
    render();
  });

  const table = h(
    'table',
    { class: 'fields-table topo-rules__table' },
    h('colgroup', null, ['layer', 'kind', 'other', 'value', 'exceptions'].map((c) => h('col', { class: `topo-rules__col--${c}` }))),
    h('thead', null, h('tr', null, ['Katman', 'Kural', 'Öbür katman', 'Değer', 'İstisna'].map((t) => h('th', { scope: 'col' }, t)))),
    body,
  );
  const dialog = new Dialog({
    title: RULES_TITLE,
    width: 860,
    className: 'dialog--io dialog--topology-rules',
    content: [
      labelled('Tolerans', h('div', { class: 'topo-rules__value topo-rules__tolerance-row' }, toleranceInput, h('span', { class: 'topo-rules__unit' }, format.lengthUnitLabel)), 'Bu uzaklıktaki iki yer aynı yerdir; bundan dar çakışma ve boşluk sayılmaz.'),
      h('div', { class: 'fields-scroll' }, table),
      h('div', { class: 'io-row fields-actions' }, add, remove),
      summary,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
    beforeClose: () => {
      if (done || !changed()) return true;
      void askUnsaved({ name: RULES_TITLE, after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat', canSave: problem() === null }).then((a) => {
        if (a === 'stay') return;
        if (a === 'save' && !write()) return;
        done = true;
        dialog.close();
      });
      return false;
    },
    onClose: () => d.dispose(),
  });
  d.add(
    listen(save, 'click', () => {
      if (!write()) return;
      done = true;
      dialog.close();
    }),
  );
  d.add(
    listen(cancel, 'click', () => {
      done = true;
      dialog.close();
    }),
  );
  render();
}
