import type { AppContext } from '../../app/context';
import type { Vec2 } from '../../model/geometry';
import { attributeFields, type BuilderObjects } from '../../model/expression/builderObjects';
import { exprCatalog } from '../../model/expression/expressionLib';
import type { InputSummary } from '../../processing/runner';
import type { FeaturesValue, LayerValue, ParamDef } from '../../processing/types';
import { h } from '../dom';
import { builderButton } from '../expression/builderApi';
import { icon } from '../icons';
import { colorSwatch } from '../layers/swatch';
import { segmented, textField, toggleSwitch } from '../widgets/controls';
import { Dropdown } from '../widgets/Dropdown';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { controlForm, enumControl } from './dialogPlan';
import { DIALOG_TEXTS as T } from './dialogTexts';
import {
  attrFieldView,
  expressionView,
  featuresView,
  insertText,
  layerFieldView,
  numberOfText,
  planLayers,
  pointView,
  previewIcon,
  toggleKind,
  withLayer,
  withScope,
} from './fieldPlan';

/**
 * One control per parameter type, generated from the definition. Typing
 * updates the value without rebuilding the form (focus stays); choices
 * that can show or hide other parameters ask for a rebuild. What each
 * control shows and does is dialogPlan's.
 */

export interface FieldEnv {
  readonly ctx: AppContext;
  /** What a features parameter resolves to ("12 kapalı alan; seçili nesneler"). */
  describe(name: string): InputSummary | undefined;
  /** How an expression parameter works out on its objects now; null when it cannot be run. */
  previewExpression(name: string): string | null;
  /** The objects an expression parameter runs on, for the expression builder. */
  builderObjects(name: string): BuilderObjects | undefined;
  /** Hides the dialog and asks for a point on the drawing. */
  pickPoint(name: string): void;
}

/** `set(value, rebuild)`: rebuild the form when the change can alter which parameters show. */
export type Setter = (value: unknown, rebuild?: boolean) => void;

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
      const form = controlForm(def);
      if (form.type === 'string' && form.short) input.classList.add('pfield__text--short');
      return input;
    }
    case 'boolean':
      return toggleSwitch({ label: def.label, checked: !!value, onChange: (v) => set(v, true) });
    case 'enum': {
      if (enumControl(def) === 'segmented') return segmented({ label: def.label, options: def.options.map((o) => ({ value: o.value, label: o.label, hint: o.hint })), value: String(value), onChange: (v) => set(v, true) });
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
    const n = numberOfText(input.value);
    input.toggleAttribute('data-invalid', Number.isNaN(n));
    set(n, false);
  });
  return h('div', { class: 'pfield__numwrap' }, input, def.unit ? h('span', { class: 'pfield__unit' }, def.unit) : null);
}

function featuresField(def: Extract<ParamDef, { type: 'features' }>, value: FeaturesValue, set: Setter, env: FieldEnv): HTMLElement {
  const layers = env.ctx.doc.layers;
  const found = env.describe(def.name);
  const view = featuresView(def, value, found, planLayers(layers));
  const form = controlForm(def);
  const seg = segmented({
    label: def.label,
    options: form.type === 'features' ? form.scopes : [],
    value: view.scope,
    onChange: (s) => {
      const next = withScope(value, s, layers.active.value);
      if (next !== value) set(next, true);
    },
  });
  const parts: HTMLElement[] = [seg];
  if (view.layer) {
    const dd = new Dropdown({
      ariaLabel: `${def.label}: katman`,
      className: 'pfield__dropdown',
      items: () =>
        (featuresView(def, value, found, planLayers(layers)).layer?.items ?? []).map((it): MenuItem => ({
          label: it.label,
          swatch: colorSwatch(layers.get(it.id)?.style.color ?? 'fg', env.ctx.view.palette),
          radio: true,
          checked: it.checked,
          run: () => set(withLayer(value, it.id), true),
        })),
    });
    dd.set(h('span', { class: 'dropdown__text' }, view.layer.text));
    parts.push(dd.el);
  }
  parts.push(h('div', { class: 'pfield__count', 'data-empty': view.count.empty ? '' : null }, icon(view.count.empty ? 'warning' : 'check', 14), h('span', null, view.count.text)));

  // Kind filter: only when there is a choice (two or more kinds in scope), or to undo one.
  const kinds = view.kinds;
  if (kinds && 'chips' in kinds) {
    const chips = kinds.chips.map((c) => {
      const b = h(
        'button',
        { class: 'pchip', type: 'button', 'aria-pressed': String(c.pressed), title: c.title },
        c.pressed ? icon('check', 12) : null,
        c.label,
        h('span', { class: 'pchip__count num' }, String(c.count)),
      );
      b.addEventListener('click', () => set(toggleKind(value, found?.byKind ?? [], c.kind), true));
      return b;
    });
    parts.push(h('div', { class: 'pfield__chips', role: 'group', 'aria-label': `${def.label}: nesne türleri` }, h('span', { class: 'pfield__chips-label' }, T.features.kinds), chips));
  } else if (kinds) {
    parts.push(h('div', { class: 'pfield__kinds' }, kinds.note));
  }
  return h('div', { class: 'pfield__stack' }, parts);
}

function layerField(def: Extract<ParamDef, { type: 'layer' }>, value: LayerValue, set: Setter, env: FieldEnv): HTMLElement {
  const layers = env.ctx.doc.layers;
  // The value as typed so far: the list and the dropdown's text follow the name field without a rebuild.
  let current = value;
  const view = () => layerFieldView(def, current, planLayers(layers));
  const dd = new Dropdown({
    ariaLabel: def.label,
    className: 'pfield__dropdown',
    items: () =>
      view().items.map((it): MenuItem => {
        if ('header' in it) return { kind: 'header', label: it.header };
        const layerId = 'layerId' in it.value ? it.value.layerId : null;
        return {
          label: it.label,
          icon: it.icon,
          swatch: layerId ? colorSwatch(layers.get(layerId)?.style.color ?? 'fg', env.ctx.view.palette) : undefined,
          radio: true,
          checked: it.checked,
          disabled: it.disabled,
          hint: it.hint,
          run: () => set(it.value, true),
        };
      }),
  });
  const first = view();
  const text = h('span', { class: 'dropdown__text' }, first.text);
  dd.set(text);
  if (first.name === null) return dd.el;
  const name = textField({
    label: `${def.label}: yeni katman adı`,
    value: first.name,
    onChange: (v) => {
      current = { newName: v };
      text.textContent = view().text;
      set(current, false);
    },
  });
  name.classList.add('pfield__text');
  return h('div', { class: 'pfield__stack' }, dd.el, name);
}

function pointField(def: Extract<ParamDef, { type: 'point' }>, value: Vec2 | null, env: FieldEnv): HTMLElement {
  const view = pointView(value, (p) => env.ctx.format.point(p));
  const pick = h('button', { class: 'btn pfield__pick', type: 'button' }, icon('snap', 14), view.button);
  pick.addEventListener('click', () => env.pickPoint(def.name));
  return h('div', { class: 'pfield__point' }, h('span', { class: `pfield__coord${view.shown ? ' num' : ''}` }, view.text), pick);
}

/** Attribute name: pick one the objects have, or (allowNew) type a new one. */
function fieldField(def: Extract<ParamDef, { type: 'field' }>, value: string, set: Setter, env: FieldEnv): HTMLElement {
  const fields = env.describe(def.of)?.fields ?? [];
  // The name as typed so far: the note and the list's tick follow it without a rebuild.
  let current = value;
  const view = () => attrFieldView(def, current, fields);
  const items = (): MenuItem[] =>
    view().items.map((it): MenuItem => ('disabled' in it ? { label: it.label, disabled: true } : { label: it.label, hint: it.hint, radio: true, checked: it.checked, run: () => set(it.label, true) }));
  const first = view();
  const note = h('div', { class: 'pfield__hint' }, first.note);
  if (!def.allowNew) {
    const dd = new Dropdown({ ariaLabel: def.label, className: 'pfield__dropdown', items });
    dd.set(h('span', { class: 'dropdown__text' }, first.text));
    return h('div', { class: 'pfield__stack' }, dd.el, note);
  }
  const input = textField({
    label: def.label,
    value,
    placeholder: T.field.placeholder,
    onChange: (v) => {
      current = v;
      note.textContent = view().note;
      set(v, false);
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
    preview.replaceChildren(...(text ? [icon(previewIcon(text), 14), h('span', null, text)] : []));
  };
  input.addEventListener('input', () => {
    set(input.value, false);
    refresh();
  });
  const insert = (text: string) => {
    const a = input.selectionStart ?? input.value.length;
    const r = insertText(input.value, a, input.selectionEnd ?? a, text);
    input.value = r.text;
    input.setSelectionRange(r.caret, r.caret);
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
  const view = expressionView(def.of ? (env.describe(def.of)?.fields ?? []) : [], null);
  const more = view.more;
  const tools = h(
    'div',
    { class: 'pfield__exprbar' },
    h(
      'div',
      { class: 'pfield__chips' },
      view.chips.length ? h('span', { class: 'pfield__chips-label' }, T.expression.fields) : null,
      view.chips.map((c) => chip(c.name, c.title, () => insert(c.token))),
      more ? menuButton(more.label, () => more.items.map((it) => ({ label: it.name, hint: it.hint, run: () => insert(it.token) }))) : null,
    ),
    h(
      'div',
      { class: 'pfield__exprmenus' },
      menuButton(T.expression.variables, () => exprCatalog().variables.map((v) => ({ label: `$${v.name}`, detail: v.description, run: () => insert(`$${v.name}`) }))),
      menuButton(T.expression.functions, () => exprCatalog().functions.map((f) => ({ label: f.signature, detail: f.description, run: () => insert(`${f.name}(`) }))),
    ),
  );
  // The expression builder (ε) edits the same text; Tamam writes it back here.
  const open = builderButton({
    get: () => input.value,
    set: (v) => {
      input.value = v;
      input.dispatchEvent(new Event('input'));
    },
    fields: () => attributeFields(def.of ? (env.describe(def.of)?.fields ?? []) : []),
    objects: () => env.builderObjects(def.name),
    context: def.label,
  });
  refresh();
  return h('div', { class: 'pfield__stack' }, h('div', { class: 'pfield__exprrow' }, input, open), tools, preview);
}
