import type { AppContext } from '../../app/context';
import type { Entity } from '../../model/entities';
import { exprCatalog } from '../../model/expression/expressionLib';
import type { Rule, SymbolSet } from '../../model/style';
import type { GeometryClass } from '../../style/geometry';
import { h, type Child } from '../dom';
import { icon } from '../icons';
import { fieldToken } from '../processing/fieldPlan';
import { PopupMenu } from '../widgets/PopupMenu';
import { symbolSetSlots } from './symbolSlot';
import { ruleCounts } from './tally';

/**
 * The rules of a rule-based layer style (QGIS "Kurala dayalı"): each rule
 * has a condition (empty: every object), a scale range, symbols and child
 * rules that narrow it; "değilse" rules catch what no sibling took. Every
 * matching rule draws. Each rule says how many drawn objects it takes, a
 * child among its parent's (./tally.ts, fixtures/style/v1/tally.json).
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

export function rulesEditor(
  ctx: AppContext,
  rules: readonly Rule[],
  classes: readonly GeometryClass[],
  entities: readonly Entity[],
  onChange: (rules: Rule[]) => void,
  layerName: (id: string) => string,
  simple?: SymbolSet,
  fields: readonly { name: string; count: number }[] = [],
): HTMLElement {
  // One call to the core per condition; geometry values from the drawing's geometry store when a condition asks.
  const counts = ruleCounts(rules, entities, { layerName, measures: (list) => ctx.view.measures(list.map((e) => e.id)) });
  const set = (path: Path, patch: Partial<Rule>) => onChange(updateAt(rules, path, (r) => ({ ...r, ...patch })));
  const block = (r: Rule, path: Path, siblings: number): Child => {
    const on = h('input', { type: 'checkbox', checked: r.enabled !== false, 'aria-label': 'Kural açık' });
    on.addEventListener('change', () => set(path, { enabled: on.checked ? undefined : false }));
    const label = h('input', { class: 'field', value: r.label, 'aria-label': 'Kural adı', spellcheck: 'false' });
    label.addEventListener('change', () => set(path, { label: label.value }));
    const filter = h('input', { class: 'field mono', value: r.filter ?? '', placeholder: r.isElse ? 'değilse: diğer kuralların almadıkları' : 'koşul yok: bütün nesneler', 'aria-label': 'Koşul', spellcheck: 'false', disabled: r.isElse });
    filter.addEventListener('change', () => set(path, { filter: filter.value.trim() || undefined }));
    // The layer's fields, the variables and the functions, put at the end of the condition.
    const insert = (text: string) => {
      const before = filter.value;
      const pad = before && !/[\s(,]$/.test(before) ? ' ' : '';
      set(path, { filter: `${before}${pad}${text}`.trim() || undefined });
    };
    const fx = h('button', { class: 'btn btn--small btn--ghost lsty__menu', type: 'button', 'aria-haspopup': 'menu', title: 'Alanlar, değişkenler ve işlevler', disabled: r.isElse }, 'ƒ', icon('chevronDown', 12));
    fx.addEventListener('click', () =>
      PopupMenu.open(
        [
          { label: 'Alanlar', items: () => (fields.length ? fields.map((f) => ({ label: f.name, hint: `${f.count} nesne`, run: () => insert(fieldToken(f.name)) })) : [{ label: 'Bu katmanın nesnelerinde öznitelik alanı yok', disabled: true }]) },
          { label: 'Değişkenler', items: () => exprCatalog().variables.map((v) => ({ label: `$${v.name}`, detail: v.description, run: () => insert(`$${v.name}`) })) },
          { label: 'İşlevler', items: () => exprCatalog().functions.map((f) => ({ label: f.signature, detail: f.description, run: () => insert(`${f.name}(`) })) },
        ],
        fx.getBoundingClientRect(),
        { owner: fx },
      ),
    );
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
    const c = counts.get(path.join('/'));
    const error = c && 'error' in c ? c.error : null;
    const title = error ?? (r.isElse ? 'Değilse kuralının aldığı nesne: üsttekinin nesnelerinden hiçbir kardeş kuralın almadıkları' : path.length > 1 ? 'Koşulu sağlayan nesne, üst kuralın aldıkları arasından' : 'Koşulu sağlayan nesne');
    return h(
      'div',
      { class: 'rule', style: `--depth:${path.length - 1}` },
      h(
        'div',
        { class: 'rule__head' },
        on,
        symbolSetSlots(ctx, r.symbols ?? {}, classes, (s) => set(path, { symbols: s }), r.label || 'Kural', simple),
        h('div', { class: 'rule__main' }, label, h('div', { class: 'rule__filter' }, filter, fx, h('label', { class: 'sdf__check rule__else' }, elseBox, h('span', null, 'değilse')))),
        h('div', { class: 'rule__scale' }, h('span', null, '1:'), scale(r.minScale, 'minScale', 'En yakın ölçek'), h('span', null, '– 1:'), scale(r.maxScale, 'maxScale', 'En uzak ölçek')),
        h('span', { class: `rule__count num${error ? ' rule__count--err' : ''}`, title }, c ? (error ? '⚠' : String('n' in c ? c.n : '')) : '—'),
        h(
          'div',
          { class: 'rule__tools' },
          tool('chevronUp', 'Yukarı', () => move(-1), i === 0),
          tool('chevronDown', 'Aşağı', () => move(1), i === siblings - 1),
          tool('plus', 'Alt kural ekle', () => set(path, { children: [...(r.children ?? []), { id: newRuleId(), label: 'Alt kural', filter: undefined, symbols: {} }] })),
          tool('trash', 'Kuralı sil', () => onChange(updateAt(rules, path, (_r, sib, k) => sib.filter((_, j) => j !== k)))),
        ),
      ),
      error ? h('div', { class: 'lsty__error' }, error) : null,
      r.enabled === false ? h('div', { class: 'lsty__help rule__off' }, 'Kapalı: bu kural ve alt kuralları çizmez.') : null,
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
    h('p', { class: 'lsty__help' }, "Koşulu sağlayan her kural çizer; alt kurallar üsttekinin nesnelerini daraltır. Ölçek aralığı boşsa her yakınlıkta görünür. Koşul örneği: Nitelik = 'Arsa' ve $alan > 500"),
    h('div', { class: 'rules' }, rules.map((r, k) => block(r, [k], rules.length))),
    h('div', { class: 'lsty__tools' }, add, addElse),
  );
}
