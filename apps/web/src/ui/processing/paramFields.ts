import type { AppContext } from '../../app/context';
import type { Vec2 } from '../../model/geometry';
import { attributeFields, type BuilderObjects } from '../../model/expression/builderObjects';
import { exprCatalog } from '../../model/expression/expressionLib';
import type { InputSummary } from '../../processing/runner';
import type { FeaturesValue, FileValue, LayerValue, NetworkValue, ParamDef, RasterPairs, RasterValues } from '../../processing/types';
import { costNames, NETWORK_KIND_LABELS } from '../../model/networkRules';
import { h } from '../dom';
import { builderButton } from '../expression/builderApi';
import { icon } from '../icons';
import { colorSwatch } from '../layers/swatch';
import { segmented, textField, toggleSwitch } from '../widgets/controls';
import { Dropdown } from '../widgets/Dropdown';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { controlForm, enumControl } from './dialogPlan';
import { DIALOG_TEXTS as T } from './dialogTexts';
import {
  attrFieldView,
  expressionView,
  featuresView,
  fileView,
  insertText,
  layerFieldView,
  numberOfText,
  PAIR_VALUES,
  pairLabel,
  planLayers,
  pointView,
  previewIcon,
  rasterPairsView,
  rasterValuesView,
  toggleKind,
  toggledName,
  withLayer,
  withoutRaster,
  withPair,
  withRasterValue,
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
  /** What a field parameter's source shown now reads: a layer's objects or a file's columns. */
  sourceOf(name: string): InputSummary | undefined;
  /** Asks for a file parameter's file and sets it (docs/adr/0200 §7). */
  chooseFile(name: string): void;
  /** How an expression parameter works out on its objects now; null when it cannot be run. */
  previewExpression(name: string): string | null;
  /** The objects an expression parameter runs on, for the expression builder. */
  builderObjects(name: string): BuilderObjects | undefined;
  /** Hides the dialog and asks for a point on the drawing. */
  pickPoint(name: string): void;
  /** Sahneden seç beside a choice that picks a point (`picks`): the point, and the choice its option. None: no button. */
  pickChoice?(name: string): void;
  /** Sahneden seç for a features field's objects, clicked or boxed on the drawing. None: no button. */
  pickObjects?(name: string): void;
}

/** `set(value, rebuild)`: rebuild the form when the change can alter which parameters show. */
export type Setter = (value: unknown, rebuild?: boolean) => void;

export function paramControl(def: ParamDef, value: unknown, set: Setter, env: FieldEnv): HTMLElement {
  switch (def.type) {
    case 'features':
      return featuresField(def, value as FeaturesValue, set, env);
    case 'number':
      return numberField(def, value as number | null, set);
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
      const control = enumField(def, value, set);
      const picks = def.picks;
      if (!picks || !env.pickChoice) return control;
      // One option is a point picked on the drawing: the pick sits beside the choice, lit while that option is chosen.
      const pick = pickButton(null, T.pick.choice, T.pick.choiceTip, () => env.pickChoice?.(def.name));
      pick.toggleAttribute('data-on', value === picks.option);
      return h('div', { class: 'pfield__pickrow' }, control, pick);
    }
    case 'layer':
      return layerField(def, value as LayerValue, set, env);
    case 'point':
      return pointField(def, value as Vec2 | null, env);
    case 'field':
      return fieldField(def, String(value ?? ''), set, env);
    case 'expression':
      return expressionField(def, String(value ?? ''), set, env);
    case 'file':
      return fileField(def, value as FileValue | null, env);
    case 'network':
      return networkField(def, value as NetworkValue | null, set, env);
    case 'rasterValues':
      return rasterValuesField(def, value as RasterValues | null, set, env);
    case 'rasterPairs':
      return rasterPairsField(def, value as RasterPairs | null, set, env);
  }
}

/** The input's raster names, none when it is not known (a model's step). */
function rastersOf(env: FieldEnv, of: string): readonly string[] | undefined {
  const found = env.describe(of);
  return found ? (found.rasters ?? []) : undefined;
}

/** A row's × for a name the input does not hold. */
function removeButton(name: string, run: () => void): HTMLButtonElement {
  const b = h('button', { class: 'btn btn--icon pfield__rowx', type: 'button', 'aria-label': `${T.rasters.remove}: ${name}` }, icon('close', 12));
  b.addEventListener('click', run);
  tooltip(b, () => ({ title: T.rasters.remove, description: T.rasters.missing }), 'top');
  return b;
}

/**
 * A value for each raster (docs/adr/0237 §9): a row a raster, its name and its field; the names the input does not hold
 * faint with ×; in a model's step, whose rasters are known only when it runs, a name can be added.
 */
function rasterValuesField(def: Extract<ParamDef, { type: 'rasterValues' }>, value: RasterValues | null, set: Setter, env: FieldEnv): HTMLElement {
  let current: RasterValues = value ?? {};
  const view = rasterValuesView(current, rastersOf(env, def.of));
  const rows = view.rows.map((r) => {
    const input = h('input', {
      class: def.cell === 'number' ? 'field pfield__num num' : 'field pfield__text',
      value: r.text,
      'aria-label': `${def.label}: ${r.name}`,
      spellcheck: 'false',
      ...(def.cell === 'number' ? { inputmode: 'decimal' } : {}),
      ...(def.placeholder ? { placeholder: def.placeholder } : {}),
    });
    input.addEventListener('input', () => {
      current = withRasterValue(def, current, r.name, input.value);
      input.toggleAttribute('data-invalid', def.cell === 'number' && !!input.value.trim() && Number.isNaN(numberOfText(input.value)));
      set(current, false);
    });
    return h(
      'div',
      { class: 'pfield__rasterrow', 'data-missing': r.listed ? null : '' },
      h('span', { class: 'pfield__rastername', title: r.name }, r.name),
      input,
      r.listed ? null : removeButton(r.name, () => set(withoutRaster(current, r.name), true)),
    );
  });
  const parts: (HTMLElement | null)[] = [...rows];
  if (view.note) parts.push(h('div', { class: 'pfield__hint' }, view.note));
  if (view.adds) {
    const name = h('input', { class: 'field pfield__text', placeholder: T.rasters.add, 'aria-label': T.rasters.add, spellcheck: 'false' });
    const add = h('button', { class: 'btn', type: 'button' }, T.rasters.addButton);
    add.addEventListener('click', () => {
      const n = name.value.trim();
      if (n && !(n in current)) set({ ...current, [n]: def.cell === 'number' ? 1 : '' }, true);
    });
    parts.push(h('div', { class: 'pfield__rasterrow' }, name, add));
  }
  return h('div', { class: 'pfield__rastertable' }, ...parts);
}

/** A comparison for each pair of rasters (İkili karşılaştırma, docs/adr/0237 §9): a row a pair, its list of 17. */
function rasterPairsField(def: Extract<ParamDef, { type: 'rasterPairs' }>, value: RasterPairs | null, set: Setter, env: FieldEnv): HTMLElement {
  const view = rasterPairsView(value, rastersOf(env, def.of));
  const rows = view.rows.map((r) => {
    const pick = new Dropdown({
      ariaLabel: `${r.a} — ${r.b}`,
      className: 'pfield__dropdown',
      items: () => PAIR_VALUES.map((v): MenuItem => ({ label: pairLabel(r.a, r.b, v), radio: true, checked: v === r.value, run: () => set(withPair(value, r.a, r.b, v), true) })),
    });
    pick.set(h('span', { class: 'dropdown__text' }, r.text));
    return h(
      'div',
      { class: 'pfield__rasterrow pfield__pairrow', 'data-missing': r.listed ? null : '' },
      h('span', { class: 'pfield__rastername', title: `${r.a} — ${r.b}` }, `${r.a} — ${r.b}`),
      pick.el,
      r.listed ? null : removeButton(`${r.a} — ${r.b}`, () => set(withPair(value, r.a, r.b, 1), true)),
    );
  });
  return h('div', { class: 'pfield__rastertable' }, ...rows, view.note ? h('div', { class: 'pfield__hint' }, view.note) : null);
}

/**
 * A network and its cost (docs/adr/0209 §10): the project's networks and the chosen one's costs (Uzunluk first), with
 * Ağlar… beside them; a project without one says so and offers Ağlar….
 */
function networkField(def: Extract<ParamDef, { type: 'network' }>, value: NetworkValue | null, set: Setter, env: FieldEnv): HTMLElement {
  const networks = env.ctx.networks.list();
  const manage = h('button', { class: 'btn pfield__pick', type: 'button' }, icon('networks', 14), 'Ağlar…');
  manage.addEventListener('click', () => env.ctx.commands.execute('network.manage'));
  if (!networks.length) return h('div', { class: 'pfield__point' }, h('span', { class: 'pfield__coord', 'data-muted': '' }, 'Projede ağ yok.'), manage);
  const chosen = networks.find((n) => n.id === value?.network) ?? networks[0];
  const costs = costNames(chosen);
  const cost = value && costs.includes(value.cost) ? value.cost : costs[0];
  const pick = new Dropdown({
    ariaLabel: def.label,
    className: 'pfield__dropdown',
    items: () => networks.map((n): MenuItem => ({ label: n.name, detail: NETWORK_KIND_LABELS[n.kind], radio: true, checked: n.id === chosen.id, run: () => set({ network: n.id, cost: costNames(n).includes(cost) ? cost : costNames(n)[0] }, true) })),
  });
  pick.set(h('span', { class: 'dropdown__text' }, chosen.name));
  const costPick = new Dropdown({
    ariaLabel: 'Maliyet',
    className: 'pfield__dropdown',
    items: () => costs.map((c): MenuItem => ({ label: c, radio: true, checked: c === cost, run: () => set({ network: chosen.id, cost: c }, true) })),
  });
  costPick.set(h('span', { class: 'dropdown__text' }, cost));
  return h('div', { class: 'pfield__pickrow pfield__network' }, pick.el, costPick.el, manage);
}

/** A file: its button, its name and what it holds (or that it must be chosen again). */
function fileField(def: Extract<ParamDef, { type: 'file' }>, value: FileValue | null, env: FieldEnv): HTMLElement {
  const view = fileView(value);
  const b = h('button', { class: 'btn pfield__pick', type: 'button' }, icon('tableFile', 14), view.button);
  b.addEventListener('click', () => env.chooseFile(def.name));
  return h(
    'div',
    { class: 'pfield__stack' },
    h('div', { class: 'pfield__point' }, h('span', { class: 'pfield__coord pfield__file', title: view.text, 'data-muted': view.chosen ? null : '' }, view.text), b),
    view.note ? h('div', { class: 'pfield__hint' }, view.note) : null,
  );
}

function enumField(def: Extract<ParamDef, { type: 'enum' }>, value: unknown, set: Setter): HTMLElement {
  if (enumControl(def) === 'segmented') return segmented({ label: def.label, options: def.options.map((o) => ({ value: o.value, label: o.label, hint: o.hint })), value: String(value), onChange: (v) => set(v, true) });
  const dd = new Dropdown({
    ariaLabel: def.label,
    className: 'pfield__dropdown',
    items: () => def.options.map((o): MenuItem => ({ label: o.label, detail: o.hint, radio: true, checked: o.value === value, run: () => set(o.value, true) })),
  });
  dd.set(h('span', { class: 'dropdown__text' }, def.options.find((o) => o.value === value)?.label ?? ''));
  return dd.el;
}

/**
 * Sahneden seç (docs/adr/0088): KentOS UI's pick, a target icon with the
 * caption, or the icon alone beside a control; its tip says what it picks.
 */
function pickButton(caption: string | null, title: string, tip: string, run: () => void): HTMLButtonElement {
  const b = h('button', { class: `btn pfield__pick${caption ? '' : ' pfield__pick--icon'}`, type: 'button', 'aria-label': caption ?? title }, icon('target', 14), caption);
  tooltip(b, () => ({ title, description: tip }), 'top');
  b.addEventListener('click', run);
  return b;
}

function numberField(def: Extract<ParamDef, { type: 'number' }>, value: number | null, set: Setter): HTMLElement {
  const input = h('input', {
    class: 'field pfield__num num',
    value: value !== null && Number.isFinite(value) ? String(value) : '',
    inputmode: 'decimal',
    'aria-label': def.label,
    spellcheck: 'false',
    ...(def.placeholder && { placeholder: def.placeholder }),
  });
  input.addEventListener('input', () => {
    // An optional field left empty is no value (docs/adr/0205 §2: the project's height).
    if (def.optional && !input.value.trim()) {
      input.removeAttribute('data-invalid');
      return set(null, false);
    }
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
  // Sahneden seç beside the scopes: objects clicked or boxed on the drawing become the selection.
  const parts: HTMLElement[] = [env.pickObjects ? h('div', { class: 'pfield__pickrow' }, seg, pickButton(T.pick.objects, T.pick.objects, T.pick.objectsTip, () => env.pickObjects?.(def.name))) : seg];
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
  const pick = h('button', { class: 'btn pfield__pick', type: 'button' }, icon('target', 14), view.button);
  pick.addEventListener('click', () => env.pickPoint(def.name));
  return h('div', { class: 'pfield__point' }, h('span', { class: `pfield__coord${view.shown ? ' num' : ''}` }, view.text), pick);
}

/** Attribute name: pick one the objects have, or (allowNew) type a new one; `multiple` checks several. */
function fieldField(def: Extract<ParamDef, { type: 'field' }>, value: string, set: Setter, env: FieldEnv): HTMLElement {
  const source = env.sourceOf(def.name);
  // The name as typed so far: the note and the list's tick follow it without a rebuild.
  let current = value;
  const view = () => attrFieldView(def, current, source);
  const items = (): MenuItem[] =>
    view().items.map((it): MenuItem =>
      'disabled' in it
        ? { label: it.label, disabled: true }
        : { label: it.label, hint: it.hint, radio: !def.multiple, checked: it.checked, run: () => set(def.multiple ? toggledName(current, it.label) : it.label, true) },
    );
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
    // İşlemler look at other layers too (docs/adr/0214 §3) and read the project's `@` values.
    variables: () => env.ctx.processing.runner.variables(),
    world: true,
    fail: (message) => env.ctx.log.error(message),
  });
  refresh();
  return h('div', { class: 'pfield__stack' }, h('div', { class: 'pfield__exprrow' }, input, open), tools, preview);
}
