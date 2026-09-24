import type { AppContext } from '../../app/context';
import { foldTurkish } from '../../core/text';
import { ENTITY_KIND_LABEL, type EntityKind } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { exprCatalog } from '../../model/expression/expressionLib';
import { scopesOf } from '../../processing/parameters';
import type { InputSummary } from '../../processing/runner';
import type { FeaturesValue, LayerValue, ParamDef } from '../../processing/types';
import { h } from '../dom';
import { icon } from '../icons';
import { colorSwatch } from '../layers/swatch';
import { segmented, textField, toggleSwitch } from '../widgets/controls';
import { Dropdown } from '../widgets/Dropdown';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';

/**
 * One control per parameter type, generated from the definition. Typing
 * updates the value without rebuilding the form (focus stays); choices
 * that can show or hide other parameters ask for a rebuild.
 */

export interface FieldEnv {
  readonly ctx: AppContext;
  /** What a features parameter resolves to now ("12 kapalı alan; seçili nesneler"). */
  describe(name: string): InputSummary | undefined;
  /** How an expression parameter works out on its objects now; null when it cannot be run. */
  previewExpression(name: string): string | null;
  /** Hides the dialog and asks for a point on the drawing. */
  pickPoint(name: string): void;
}

/** `set(value, rebuild)`: rebuild the form when the change can alter which parameters show. */
export type Setter = (value: unknown, rebuild?: boolean) => void;

const SCOPE_SHORT = { selection: 'Seçili', visible: 'Görünen', all: 'Tümü', layer: 'Katman' } as const;
const kindLabel = (k: EntityKind) => ENTITY_KIND_LABEL[k];

export function paramControl(def: ParamDef, value: unknown, set: Setter, env: FieldEnv): HTMLElement {
  switch (def.type) {
    case 'features':
      return featuresField(def, value as FeaturesValue, set, env);
    case 'number':
      return numberField(def, value as number, set);
    case 'string': {
      const input = textField({ label: def.label, value: String(value ?? ''), placeholder: def.placeholder, onChange: (v) => set(v, false) });
      if (def.maxLength) input.maxLength = def.maxLength;
      input.classList.add('pfield__text');
      if (def.maxLength && def.maxLength <= 2) input.classList.add('pfield__text--short');
      return input;
    }
    case 'boolean':
      return toggleSwitch({ label: def.label, checked: !!value, onChange: (v) => set(v, true) });
    case 'enum': {
      const short = def.options.length <= 3 && def.options.every((o) => o.label.length <= 22);
      if (short) return segmented({ label: def.label, options: def.options.map((o) => ({ value: o.value, label: o.label, hint: o.hint })), value: String(value), onChange: (v) => set(v, true) });
      const dd = new Dropdown({
        ariaLabel: def.label,
        className: 'pfield__dropdown',
        items: () => def.options.map((o): MenuItem => ({ label: o.label, detail: o.hint, radio: true, checked: o.value === value, run: () => set(o.value, true) })),
      });
      dd.set(h('span', { class: 'dropdown__text' }, def.options.find((o) => o.value === value)?.label ?? ''));
      return dd.el;
    }
    case 'layer':
      return layerField(def, value as LayerValue, set, env);
    case 'point':
      return pointField(def, value as Vec2 | null, env);
    case 'field':
      return fieldField(def, String(value ?? ''), set, env);
    case 'expression':
      return expressionField(def, String(value ?? ''), set, env);
  }
}

function numberField(def: Extract<ParamDef, { type: 'number' }>, value: number, set: Setter): HTMLElement {
  const input = h('input', { class: 'field pfield__num num', value: Number.isFinite(value) ? String(value) : '', inputmode: 'decimal', 'aria-label': def.label, spellcheck: 'false' });
  input.addEventListener('input', () => {
    const n = Number(input.value.replace(',', '.'));
    const ok = input.value.trim() !== '' && Number.isFinite(n);
    input.toggleAttribute('data-invalid', !ok);
    set(ok ? n : NaN, false);
  });
  return h('div', { class: 'pfield__numwrap' }, input, def.unit ? h('span', { class: 'pfield__unit' }, def.unit) : null);
}

function featuresField(def: Extract<ParamDef, { type: 'features' }>, value: FeaturesValue, set: Setter, env: FieldEnv): HTMLElement {
  const scopes = scopesOf(def);
  const layers = env.ctx.doc.layers;
  const kinds = value.kinds;
  const seg = segmented({
    label: def.label,
    options: scopes.map((s) => ({ value: s, label: SCOPE_SHORT[s] })),
    value: value.scope === 'ids' ? scopes[0] : value.scope,
    onChange: (s) => set(s === 'layer' ? { scope: 'layer', layerId: layers.active.value, kinds } : { scope: s, kinds }, true),
  });
  const parts: HTMLElement[] = [seg];
  if (value.scope === 'layer') {
    const dd = new Dropdown({
      ariaLabel: `${def.label}: katman`,
      className: 'pfield__dropdown',
      items: () =>
        layers.leaves().map((l): MenuItem => ({
          label: layers.path(l.id),
          swatch: colorSwatch(l.style.color, env.ctx.view.palette),
          radio: true,
          checked: l.id === value.layerId,
          run: () => set({ scope: 'layer', layerId: l.id, kinds }, true),
        })),
    });
    dd.set(h('span', { class: 'dropdown__text' }, layers.get(value.layerId)?.name ?? '—'));
    parts.push(dd.el);
  }
  const found = env.describe(def.name);
  const empty = !found?.count;
  parts.push(h('div', { class: 'pfield__count', 'data-empty': empty ? '' : null }, icon(empty ? 'warning' : 'check', 14), h('span', null, found?.description ?? '')));

  // Kind filter: only when there is a choice (two or more kinds in scope), or to undo one.
  const present = found?.byKind ?? [];
  if (present.length >= 2 || (kinds && present.length)) {
    const on = new Set<EntityKind>(kinds ?? present.map((k) => k.kind));
    const chips = present.map((k) => {
      const pressed = on.has(k.kind);
      const b = h(
        'button',
        { class: 'pchip', type: 'button', 'aria-pressed': String(pressed), title: pressed ? 'Bu türü dışarıda bırak' : 'Bu türü de al' },
        pressed ? icon('check', 12) : null,
        kindLabel(k.kind),
        h('span', { class: 'pchip__count num' }, String(k.count)),
      );
      b.addEventListener('click', () => {
        const next = new Set(on);
        if (pressed) next.delete(k.kind);
        else next.add(k.kind);
        const all = present.every((p) => next.has(p.kind));
        set({ ...value, kinds: all ? undefined : present.map((p) => p.kind).filter((x) => next.has(x)) }, true);
      });
      return b;
    });
    parts.push(h('div', { class: 'pfield__chips', role: 'group', 'aria-label': `${def.label}: nesne türleri` }, h('span', { class: 'pfield__chips-label' }, 'Türler'), chips));
  } else if (def.kinds) {
    parts.push(h('div', { class: 'pfield__kinds' }, `Uygun nesneler: ${def.kinds.map((k) => kindLabel(k).toLocaleLowerCase('tr-TR')).join(', ')}`));
  }
  return h('div', { class: 'pfield__stack' }, parts);
}

function layerField(def: Extract<ParamDef, { type: 'layer' }>, value: LayerValue, set: Setter, env: FieldEnv): HTMLElement {
  const layers = env.ctx.doc.layers;
  const suggested = (() => {
    const d = typeof def.default === 'function' ? null : def.default;
    return d && 'newName' in d ? d.newName : 'Yeni katman';
  })();
  const isNew = 'newName' in value;
  const dd = new Dropdown({
    ariaLabel: def.label,
    className: 'pfield__dropdown',
    items: () => [
      { kind: 'header', label: 'Yeni katman' },
      { label: `Yeni: ${isNew ? value.newName : suggested}`, icon: 'layerAdd', radio: true, checked: isNew, run: () => set({ newName: isNew ? value.newName : suggested }, true) },
      { kind: 'header', label: 'Mevcut katmanlar' },
      ...layers.leaves().map(
        (l): MenuItem => ({
          label: layers.path(l.id),
          swatch: colorSwatch(l.style.color, env.ctx.view.palette),
          radio: true,
          checked: !isNew && value.layerId === l.id,
          disabled: layers.isLocked(l.id),
          hint: layers.isLocked(l.id) ? 'kilitli' : undefined,
          run: () => set({ layerId: l.id }, true),
        }),
      ),
    ],
  });
  if (isNew) {
    const existing = layers.leaves().some((l) => l.name.toLocaleLowerCase('tr-TR') === value.newName.trim().toLocaleLowerCase('tr-TR'));
    dd.set(h('span', { class: 'dropdown__text' }, existing ? `${value.newName} (mevcut)` : `${value.newName} (yeni)`));
    const name = textField({ label: `${def.label}: yeni katman adı`, value: value.newName, onChange: (v) => set({ newName: v }, false) });
    name.classList.add('pfield__text');
    return h('div', { class: 'pfield__stack' }, dd.el, name);
  }
  dd.set(h('span', { class: 'dropdown__text' }, layers.get(value.layerId)?.name ?? '—'));
  return dd.el;
}

function pointField(def: Extract<ParamDef, { type: 'point' }>, value: Vec2 | null, env: FieldEnv): HTMLElement {
  const pick = h('button', { class: 'btn pfield__pick', type: 'button' }, icon('snap', 14), value ? 'Yeniden göster' : 'Haritadan göster');
  pick.addEventListener('click', () => env.pickPoint(def.name));
  return h('div', { class: 'pfield__point' }, h('span', { class: `pfield__coord${value ? ' num' : ''}` }, value ? env.ctx.format.point(value) : 'Henüz gösterilmedi'), pick);
}

/** Attribute name: pick one the objects have, or (allowNew) type a new one. */
function fieldField(def: Extract<ParamDef, { type: 'field' }>, value: string, set: Setter, env: FieldEnv): HTMLElement {
  const fields = env.describe(def.of)?.fields ?? [];
  const items = (): MenuItem[] =>
    fields.length
      ? fields.map((f) => ({ label: f.name, hint: `${f.count} nesne`, radio: true, checked: f.name === value, run: () => set(f.name, true) }))
      : [{ label: 'Bu nesnelerde öznitelik alanı yok', disabled: true }];
  const note = h('div', { class: 'pfield__hint' });
  const describeName = (name: string) => {
    const known = fields.find((f) => f.name === name.trim());
    note.textContent = name.trim() ? (known ? `${known.count} nesnede var; değeri değişir.` : def.allowNew ? 'Yeni alan: nesnelere eklenir.' : 'Bu nesnelerde böyle bir alan yok.') : '';
  };
  describeName(value);
  if (!def.allowNew) {
    const dd = new Dropdown({ ariaLabel: def.label, className: 'pfield__dropdown', items });
    dd.set(h('span', { class: 'dropdown__text' }, value || 'Alan seçin'));
    return h('div', { class: 'pfield__stack' }, dd.el, note);
  }
  const input = textField({
    label: def.label,
    value,
    placeholder: 'Alan adı',
    onChange: (v) => {
      set(v, false);
      describeName(v);
    },
  });
  input.classList.add('pfield__combo-input');
  const open = h('button', { class: 'pfield__combo-btn', type: 'button', 'aria-label': `${def.label}: var olan alanlar`, 'aria-haspopup': 'listbox' }, icon('chevronDown', 14));
  const combo = h('div', { class: 'pfield__combo' }, input, open);
  open.addEventListener('click', () => {
    const r = combo.getBoundingClientRect();
    PopupMenu.open(items(), r, { owner: open, minWidth: r.width });
  });
  return h('div', { class: 'pfield__stack' }, combo, note);
}

const PLAIN_NAME = /^[\p{L}_][\p{L}\p{N}_]*$/u;
const RESERVED = new Set(['VE', 'VEYA', 'DEGIL', 'AND', 'OR', 'NOT', 'DOGRU', 'YANLIS', 'TRUE', 'FALSE', 'BOS', 'NULL']);

/** How a field is written in an expression: bare when it can be, in brackets otherwise. */
const fieldToken = (name: string) => (PLAIN_NAME.test(name) && !RESERVED.has(foldTurkish(name)) ? name : `[${name}]`);

/** Fields shown as chips; the rest sit behind a "+n" menu. */
const CHIP_FIELDS = 6;

/**
 * Expression editor: one line (mono, it is typed like the command line),
 * fields of the input as chips, variables and functions from menus that
 * explain each one, and a live line saying what it does to the objects.
 */
function expressionField(def: Extract<ParamDef, { type: 'expression' }>, value: string, set: Setter, env: FieldEnv): HTMLElement {
  const input = h('input', { class: 'field pfield__expr', value, placeholder: def.placeholder ?? '', 'aria-label': def.label, spellcheck: 'false', autocomplete: 'off' });
  const preview = h('div', { class: 'pfield__preview' });
  const refresh = () => {
    const text = env.previewExpression(def.name);
    preview.replaceChildren(...(text ? [icon(/ yok\.| boş\./.test(text) ? 'info' : 'check', 14), h('span', null, text)] : []));
  };
  input.addEventListener('input', () => {
    set(input.value, false);
    refresh();
  });
  const insert = (text: string) => {
    const a = input.selectionStart ?? input.value.length;
    const b = input.selectionEnd ?? a;
    const before = input.value.slice(0, a);
    const pad = before && !/[\s(,]$/.test(before) ? ' ' : '';
    input.setRangeText(pad + text, a, b, 'end');
    input.focus();
    input.dispatchEvent(new Event('input'));
  };
  // Buttons do not take focus, so the caret stays where the text goes.
  const keepCaret = (b: HTMLElement) => b.addEventListener('mousedown', (e) => e.preventDefault());
  const chip = (label: string, title: string, run: () => void) => {
    const b = h('button', { class: 'pchip pchip--insert', type: 'button', title }, label);
    keepCaret(b);
    b.addEventListener('click', run);
    return b;
  };
  const menuButton = (label: string, items: () => MenuItem[]) => {
    const b = h('button', { class: 'pchip pchip--menu', type: 'button', 'aria-haspopup': 'menu' }, label, icon('chevronDown', 12));
    keepCaret(b);
    b.addEventListener('click', () => PopupMenu.open(items(), b.getBoundingClientRect(), { owner: b }));
    return b;
  };
  const fields = def.of ? (env.describe(def.of)?.fields ?? []) : [];
  const extra = fields.slice(CHIP_FIELDS);
  const tools = h(
    'div',
    { class: 'pfield__exprbar' },
    h(
      'div',
      { class: 'pfield__chips' },
      fields.length ? h('span', { class: 'pfield__chips-label' }, 'Alanlar') : null,
      fields.slice(0, CHIP_FIELDS).map((f) => chip(f.name, `${f.count} nesnede var; ifadeye ekle`, () => insert(fieldToken(f.name)))),
      extra.length ? menuButton(`+${extra.length}`, () => extra.map((f) => ({ label: f.name, hint: `${f.count}`, run: () => insert(fieldToken(f.name)) }))) : null,
    ),
    h(
      'div',
      { class: 'pfield__exprmenus' },
      menuButton('Değişkenler', () => exprCatalog().variables.map((v) => ({ label: `$${v.name}`, detail: v.description, run: () => insert(`$${v.name}`) }))),
      menuButton('İşlevler', () => exprCatalog().functions.map((f) => ({ label: f.signature, detail: f.description, run: () => insert(`${f.name}(`) }))),
    ),
  );
  refresh();
  return h('div', { class: 'pfield__stack' }, input, tools, preview);
}
