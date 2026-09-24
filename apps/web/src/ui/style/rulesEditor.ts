import type { AppContext } from '../../app/context';
import type { Entity } from '../../model/entities';
import { compileExpression } from '../../model/expression/expression';
import { measuredOf, truthy } from '../../model/expression/expressionLib';
import type { Rule } from '../../model/style';
import type { GeometryClass } from '../../style/geometry';
import { h, type Child } from '../dom';
import { icon } from '../icons';
import { symbolSetSlots } from './symbolSlot';

/**
 * The rules of a rule-based layer style (QGIS "Kurala dayalı"): each rule
 * has a condition (empty: every object), a scale range, symbols and child
 * rules that narrow it; "değilse" rules catch what no sibling took. Every
 * matching rule draws. Counts show how many objects each condition takes.
 */

let seq = 0;
const newRuleId = () => `r${Date.now().toString(36)}${(seq++).toString(36)}`;

type Path = readonly number[];

function updateAt(rules: readonly Rule[], path: Path, fn: (r: Rule, siblings: Rule[], i: number) => Rule[] | Rule): Rule[] {
  const [i, ...rest] = path;
  if (!rest.length) {
    const copy = [...rules];
    const res = fn(copy[i], copy, i);
    if (Array.isArray(res)) return res;
    copy[i] = res;
    return copy;
  }
  return rules.map((r, k) => (k === i ? { ...r, children: updateAt(r.children ?? [], rest, fn) } : r));
}

export function rulesEditor(ctx: AppContext, rules: readonly Rule[], classes: readonly GeometryClass[], entities: readonly Entity[], onChange: (rules: Rule[]) => void, layerName: (id: string) => string): HTMLElement {
  const count = (filter: string | undefined): { n: number; error?: string } => {
    if (!filter?.trim()) return { n: entities.length };
    const c = compileExpression(filter);
    if (!c.ok) return { n: 0, error: c.error };
    // Geometry values from the drawing's geometry store, for all the objects at once when the filter first asks.
    const measured = measuredOf(() => ctx.view.measures(entities.map((e) => e.id)));
    let n = 0;
    entities.forEach((entity, i) => {
      if (truthy(c.expr.evaluate({ entity, index: i + 1, layerName, measured: () => measured(i) }))) n++;
    });
    return { n };
  };
  const set = (path: Path, patch: Partial<Rule>) => onChange(updateAt(rules, path, (r) => ({ ...r, ...patch })));
  const block = (r: Rule, path: Path, siblings: number): Child => {
    const on = h('input', { type: 'checkbox', checked: r.enabled !== false, 'aria-label': 'Kural açık' });
    on.addEventListener('change', () => set(path, { enabled: on.checked ? undefined : false }));
    const label = h('input', { class: 'field', value: r.label, 'aria-label': 'Kural adı', spellcheck: 'false' });
    label.addEventListener('change', () => set(path, { label: label.value }));
    const filter = h('input', { class: 'field mono', value: r.filter ?? '', placeholder: r.isElse ? 'değilse: diğer kuralların almadıkları' : 'koşul yok: bütün nesneler', 'aria-label': 'Koşul', spellcheck: 'false', disabled: r.isElse });
    filter.addEventListener('change', () => set(path, { filter: filter.value.trim() || undefined }));
    const scale = (value: number | undefined, key: 'minScale' | 'maxScale', label: string) => {
      const el = h('input', { class: 'field num lsty__scale', value: value ? String(value) : '', placeholder: key === 'minScale' ? 'en yakın' : 'en uzak', inputmode: 'numeric', 'aria-label': label });
      el.addEventListener('change', () => {
        const v = Number(el.value.replace(/[.\s]/g, '').replace(',', '.'));
        set(path, { [key]: el.value.trim() && Number.isFinite(v) && v > 0 ? v : undefined });
      });
      return el;
    };
    const elseBox = h('input', { type: 'checkbox', checked: !!r.isElse, 'aria-label': 'Değilse kuralı' });
    elseBox.addEventListener('change', () => set(path, { isElse: elseBox.checked || undefined, filter: elseBox.checked ? undefined : r.filter }));
    const tool = (name: string, label: string, run: () => void, disabled = false) => {
      const b = h('button', { class: 'ibtn', type: 'button', 'aria-label': label, title: label, disabled }, icon(name, 15));
      b.addEventListener('click', run);
      return b;
    };
    const i = path[path.length - 1];
    const move = (d: number) =>
      onChange(
        updateAt(rules, path, (_r, sib, k) => {
          const to = k + d;
          [sib[k], sib[to]] = [sib[to], sib[k]];
          return sib;
        }),
      );
    const c = r.isElse ? null : count(r.filter);
    return h(
      'div',
      { class: 'rule', style: `--depth:${path.length - 1}` },
      h(
        'div',
        { class: 'rule__head' },
        on,
        symbolSetSlots(ctx, r.symbols ?? {}, classes, (s) => set(path, { symbols: s }), r.label || 'Kural'),
        h('div', { class: 'rule__main' }, label, h('div', { class: 'rule__filter' }, filter, h('label', { class: 'sdf__check rule__else' }, elseBox, h('span', null, 'değilse')))),
        h('div', { class: 'rule__scale' }, h('span', null, '1:'), scale(r.minScale, 'minScale', 'En yakın ölçek'), h('span', null, '– 1:'), scale(r.maxScale, 'maxScale', 'En uzak ölçek')),
        h('span', { class: `rule__count num${c?.error ? ' rule__count--err' : ''}`, title: c?.error ?? 'Koşulu sağlayan nesne' }, c ? (c.error ? '⚠' : String(c.n)) : '—'),
        h(
          'div',
          { class: 'rule__tools' },
          tool('chevronUp', 'Yukarı', () => move(-1), i === 0),
          tool('chevronDown', 'Aşağı', () => move(1), i === siblings - 1),
          tool('plus', 'Alt kural ekle', () => set(path, { children: [...(r.children ?? []), { id: newRuleId(), label: 'Alt kural', filter: undefined, symbols: {} }] })),
          tool('trash', 'Kuralı sil', () => onChange(updateAt(rules, path, (_r, sib, k) => sib.filter((_, j) => j !== k)))),
        ),
      ),
      c?.error ? h('div', { class: 'lsty__error' }, c.error) : null,
      (r.children ?? []).map((child, k) => block(child, [...path, k], r.children!.length)),
    );
  };
  const add = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), 'Kural ekle');
  add.addEventListener('click', () => onChange([...rules, { id: newRuleId(), label: 'Yeni kural', filter: undefined, symbols: {} }]));
  const addElse = h('button', { class: 'btn btn--small btn--ghost', type: 'button' }, 'Değilse kuralı ekle');
  addElse.addEventListener('click', () => onChange([...rules, { id: newRuleId(), label: 'Diğerleri', isElse: true, symbols: {} }]));
  return h(
    'div',
    { class: 'lsty__panel' },
    h('p', { class: 'lsty__help' }, 'Koşulu sağlayan her kural çizer; alt kurallar üsttekinin nesnelerini daraltır. Ölçek aralığı boşsa her yakınlıkta görünür. Koşul örneği: "Nitelik" = \'Arsa\' ve $alan > 500'),
    h('div', { class: 'rules' }, rules.map((r, k) => block(r, [k], rules.length))),
    h('div', { class: 'lsty__tools' }, add, addElse),
  );
}
