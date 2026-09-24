import type { AppContext } from '../../../app/context';
import { ENTITY_KIND_LABEL, type EntityKind } from '../../../model/entities';
import { stepName, type ModelIssue, type ProcessingModel, type ValueSource } from '../../../processing/model';
import { addInput, INPUT_TYPES, removeInput, removeStep, setSource, sourcesFor, type ModelInputType } from '../../../processing/modelEdit';
import { defaultValue, isVisible } from '../../../processing/parameters';
import type { FeaturesValue, ParamDef, ProcessingTool } from '../../../processing/types';
import { h } from '../../dom';
import { icon } from '../../icons';
import { segmented, textField, toggleSwitch } from '../../widgets/controls';
import { Dropdown } from '../../widgets/Dropdown';
import type { MenuItem } from '../../widgets/PopupMenu';
import { paramControl, type FieldEnv } from '../paramFields';
import type { NodeRef } from './ModelCanvas';

/**
 * Right column of the model designer: the model's name, category,
 * outputs and problems when nothing is selected; an input's definition;
 * or a step's parameters, each fed by a fixed value, the tool's default,
 * a model input or an earlier step's output.
 */

export interface InspectorHost {
  readonly ctx: AppContext;
  readonly model: ProcessingModel;
  readonly problems: readonly ModelIssue[];
  readonly saved: boolean;
  readonly builtinCopy: boolean;
  lookup(id: string): ProcessingTool | undefined;
  /** Changes the draft. `rerender: false` while typing (focus stays); `key` joins typing into one undo step. */
  change(fn: () => void, opts?: { rerender?: boolean; key?: string }): void;
  select(ref: NodeRef | null): void;
  pickPoint(stepId: string, param: string): void;
  deleteModel(): void;
}

const KIND_CHOICES: readonly EntityKind[] = ['polygon', 'polyline', 'line', 'point', 'circle', 'arc', 'text'];

const section = (title: string | null, ...children: (Node | null)[]) => h('section', { class: 'mins__section' }, title ? h('div', { class: 'mins__title' }, title) : null, children);
const row = (label: string, control: Node, note?: string | null) => h('div', { class: 'mins__row' }, h('div', { class: 'mins__label' }, label), control, note ? h('div', { class: 'mins__note' }, note) : null);

export function renderInspector(host: InspectorHost, sel: NodeRef | null): HTMLElement {
  if (sel?.kind === 'input') {
    const def = host.model.inputs.find((i) => i.name === sel.name);
    if (def) return inputInspector(host, def);
  }
  if (sel?.kind === 'step') {
    const step = host.model.steps.find((s) => s.id === sel.id);
    if (step) return stepInspector(host, step.id);
  }
  return modelInspector(host);
}

// ── The model ──────────────────────────────────────────────────────────

function modelInspector(host: InspectorHost): HTMLElement {
  const { model, ctx } = host;
  const registry = ctx.processing.registry;
  const categories = registry.tree().map((n) => n.category);
  const all = [...new Map([...categories, ...(registry.category(model.category) ? [registry.category(model.category)!] : [])].map((c) => [c.id, c])).values()];
  const category = new Dropdown({
    ariaLabel: 'Kategori',
    className: 'pfield__dropdown',
    items: () => all.map((c): MenuItem => ({ label: c.label, icon: c.icon, radio: true, checked: c.id === model.category, run: () => host.change(() => (model.category = c.id)) })),
  });
  category.set(h('span', { class: 'dropdown__text' }, registry.category(model.category)?.label ?? model.category));

  const outputs = model.outputs.map((o) => {
    const step = model.steps.find((s) => s.id === o.from.step);
    const out = step && host.lookup(step.tool)?.outputs?.find((x) => x.name === o.from.output);
    const remove = h('button', { class: 'ibtn', type: 'button', 'aria-label': `“${o.label}” çıktısını kaldır` }, icon('close', 14));
    remove.addEventListener('click', () => host.change(() => (model.outputs = model.outputs.filter((x) => x !== o))));
    return h('div', { class: 'mins__item' }, h('span', { class: 'mins__item-main' }, h('b', null, o.label), h('span', null, step ? `${stepName(step, host.lookup)} › ${out?.label ?? o.from.output}` : 'adım yok')), remove);
  });
  const candidates = model.steps.flatMap((s) =>
    (host.lookup(s.tool)?.outputs ?? []).filter((o) => !model.outputs.some((x) => x.from.step === s.id && x.from.output === o.name)).map((o) => ({ step: s, out: o })),
  );
  const addOutput = new Dropdown({
    ariaLabel: 'Çıktı ekle',
    className: 'pfield__dropdown',
    items: () =>
      candidates.length
        ? candidates.map(({ step, out }): MenuItem => ({
            label: out.label,
            detail: stepName(step, host.lookup),
            run: () =>
              host.change(() => {
                const name = out.name in Object.fromEntries(model.outputs.map((x) => [x.name, 1])) ? `${out.name}${model.outputs.length + 1}` : out.name;
                model.outputs.push({ name, label: out.label, from: { step: step.id, output: out.name } });
              }),
          }))
        : [{ label: 'Eklenebilecek çıktı yok', disabled: true }],
  });
  addOutput.set(h('span', { class: 'dropdown__text' }, 'Çıktı ekle'));

  const del = host.saved && !host.builtinCopy ? h('button', { class: 'btn btn--ghost mins__danger', type: 'button' }, icon('erase', 14), 'Modeli sil') : null;
  del?.addEventListener('click', () => host.deleteModel());

  return h(
    'div',
    { class: 'mins' },
    h('div', { class: 'mins__head' }, h('span', { class: 'mins__icon' }, icon('processing', 18)), h('div', null, h('div', { class: 'mins__kind' }, 'Model'), h('div', { class: 'mins__lead' }, 'Adı ve açıklaması araç kutusunda ve menüde görünür.'))),
    section(
      null,
      row('Ad', textField({ label: 'Model adı', value: model.label, onChange: (v) => host.change(() => (model.label = v), { rerender: false, key: 'label' }) })),
      row('Kategori', category.el),
      row('Açıklama', textField({ label: 'Model açıklaması', value: model.description, placeholder: 'Ne yapar, tek cümle', onChange: (v) => host.change(() => (model.description = v), { rerender: false, key: 'description' }) })),
    ),
    section('Model çıktıları', ...outputs, addOutput.el),
    problemsSection(host, null),
    del ? section(null, del) : null,
  );
}

function problemsSection(host: InspectorHost, stepId: string | null): HTMLElement | null {
  const list = host.problems.filter((p) => stepId === null || p.step === stepId);
  if (!list.length) return stepId === null ? section(null, h('div', { class: 'mins__ok' }, icon('check', 14), host.model.steps.length ? 'Model çalışmaya hazır.' : 'Başlamak için soldan bir girdi ve bir araç ekleyin.')) : null;
  return section(
    stepId ? 'Sorunlar' : `Sorunlar (${list.length})`,
    ...list.map((p) => {
      const step = p.step ? host.model.steps.find((s) => s.id === p.step) : null;
      const b = h('button', { class: 'mins__problem', type: 'button' }, icon('warning', 14), h('span', null, step && !stepId ? `${stepName(step, host.lookup)}: ${p.message}` : p.message));
      if (step && !stepId) b.addEventListener('click', () => host.select({ kind: 'step', id: step.id }));
      return b;
    }),
  );
}

// ── An input ───────────────────────────────────────────────────────────

function inputInspector(host: InspectorHost, def: ParamDef): HTMLElement {
  const type = INPUT_TYPES.find((t) => t.type === def.type);
  const d = def as ParamDef & { default?: unknown; description?: string; optional?: boolean };
  const set = <K extends string>(key: K, value: unknown, rerender = true) => host.change(() => Object.assign(def, { [key]: value }), { rerender, key: `${def.name}.${key}` });
  const users = host.model.steps.filter((s) => Object.values(s.values).some((v) => v.kind === 'input' && v.name === def.name));

  const specific: (Node | null)[] = [];
  if (def.type === 'features') {
    const dv = (d.default as FeaturesValue | undefined) ?? { scope: 'selection' };
    specific.push(
      row(
        'Varsayılan',
        segmented({
          label: 'Varsayılan kapsam',
          options: [
            { value: 'selection', label: 'Seçili' },
            { value: 'visible', label: 'Görünen' },
            { value: 'all', label: 'Tümü' },
          ],
          value: dv.scope === 'selection' || dv.scope === 'visible' || dv.scope === 'all' ? dv.scope : 'selection',
          onChange: (v) => set('default', { scope: v }),
        }),
      ),
    );
    const kinds = new Set(def.kinds ?? []);
    const chips = KIND_CHOICES.map((k) => {
      const on = kinds.has(k);
      const b = h('button', { class: 'pchip', type: 'button', 'aria-pressed': String(on) }, on ? icon('check', 12) : null, ENTITY_KIND_LABEL[k]);
      b.addEventListener('click', () => {
        const next = new Set(kinds);
        if (on) next.delete(k);
        else next.add(k);
        set('kinds', next.size ? KIND_CHOICES.filter((x) => next.has(x)) : undefined);
      });
      return b;
    });
    specific.push(row('Uygun nesneler', h('div', { class: 'pfield__chips' }, chips), kinds.size ? null : 'Hiçbiri seçili değilse her tür alınır; adımlar kendi türlerini ayrıca süzer.'));
  } else if (def.type === 'number') {
    const num = (label: string, key: 'default' | 'min' | 'max', value: number | undefined, optional: boolean) => {
      const input = h('input', { class: 'field pfield__num num', value: value === undefined ? '' : String(value), inputmode: 'decimal', 'aria-label': label, placeholder: optional ? 'yok' : '' });
      input.addEventListener('input', () => {
        const t = input.value.trim().replace(',', '.');
        const n = Number(t);
        if (t === '' && optional) set(key, undefined, false);
        else if (t !== '' && Number.isFinite(n)) set(key, n, false);
      });
      return row(label, input);
    };
    specific.push(
      num('Varsayılan', 'default', typeof d.default === 'number' ? d.default : 0, false),
      num('En az', 'min', def.min, true),
      num('En çok', 'max', def.max, true),
      row('Tam sayı', toggleSwitch({ label: 'Tam sayı', checked: !!def.integer, onChange: (v) => set('integer', v || undefined) })),
    );
  } else if (def.type === 'string') {
    specific.push(
      row('Varsayılan', textField({ label: 'Varsayılan metin', value: String(d.default ?? ''), onChange: (v) => set('default', v, false) })),
      row('Boş bırakılabilir', toggleSwitch({ label: 'Boş bırakılabilir', checked: !!def.allowEmpty, onChange: (v) => set('allowEmpty', v || undefined) })),
    );
  } else if (def.type === 'boolean') {
    specific.push(row('Varsayılan', toggleSwitch({ label: 'Varsayılan', checked: !!d.default, onChange: (v) => set('default', v) })));
  } else if (def.type === 'layer') {
    const dv = d.default as { newName?: string } | undefined;
    specific.push(row('Varsayılan yeni katman', textField({ label: 'Varsayılan yeni katman adı', value: dv?.newName ?? '', onChange: (v) => set('default', { newName: v }, false) }), 'Çalıştırırken var olan bir katman da seçilebilir.'));
  } else if (def.type === 'point') {
    specific.push(h('div', { class: 'mins__note' }, 'Çalıştırırken haritada gösterilir ya da Y,X yazılır.'));
  }

  const del = h('button', { class: 'btn btn--ghost mins__danger', type: 'button' }, icon('erase', 14), 'Girdiyi sil');
  del.addEventListener('click', () =>
    host.change(() => {
      removeInput(host.model, def.name);
      host.select(null);
    }),
  );

  return h(
    'div',
    { class: 'mins' },
    h('div', { class: 'mins__head' }, h('span', { class: 'mins__icon mins__icon--input' }, icon(type?.icon ?? 'processing', 18)), h('div', null, h('div', { class: 'mins__kind' }, `Girdi: ${type?.label ?? def.type}`), h('div', { class: 'mins__lead' }, 'Model çalıştırılırken kullanıcıdan istenir.'))),
    section(
      null,
      row('Etiket', textField({ label: 'Girdi etiketi', value: def.label, onChange: (v) => set('label', v, false) }), `Değişken adı: ${def.name}`),
      row('Açıklama', textField({ label: 'Girdi açıklaması', value: d.description ?? '', placeholder: 'Pencerede etiketin altında görünür', onChange: (v) => set('description', v || undefined, false) })),
      row('İsteğe bağlı', toggleSwitch({ label: 'İsteğe bağlı', checked: !!d.optional, onChange: (v) => set('optional', v || undefined) })),
      ...specific,
    ),
    section(
      'Kullanan adımlar',
      ...(users.length
        ? users.map((s) => {
            const b = h('button', { class: 'mins__link', type: 'button' }, icon(host.lookup(s.tool)?.icon ?? 'processing', 14), stepName(s, host.lookup));
            b.addEventListener('click', () => host.select({ kind: 'step', id: s.id }));
            return b;
          })
        : [h('div', { class: 'mins__note' }, 'Henüz hiçbir adım bu girdiyi kullanmıyor. Kutunun sağındaki noktadan bir adıma sürükleyin.')]),
    ),
    section(null, del),
  );
}

// ── A step ─────────────────────────────────────────────────────────────

/** The model input type a parameter can become (none for choices from a fixed list). */
function inputTypeFor(p: ParamDef): ModelInputType | null {
  switch (p.type) {
    case 'features':
    case 'number':
    case 'string':
    case 'boolean':
    case 'layer':
    case 'point':
      return p.type;
    case 'expression':
    case 'field':
      return 'string';
    default:
      return null;
  }
}

function sourceText(host: InspectorHost, src: ValueSource | undefined): string {
  if (!src) return 'Aracın varsayılanı';
  if (src.kind === 'value') return 'Sabit değer';
  if (src.kind === 'input') return `Girdi: ${host.model.inputs.find((i) => i.name === src.name)?.label ?? src.name}`;
  const step = host.model.steps.find((s) => s.id === src.step);
  const out = step && host.lookup(step.tool)?.outputs?.find((o) => o.name === src.output);
  return `${step ? stepName(step, host.lookup) : src.step} › ${out?.label ?? src.output}`;
}

function stepInspector(host: InspectorHost, stepId: string): HTMLElement {
  const { model, ctx } = host;
  const step = model.steps.find((s) => s.id === stepId)!;
  const tool = host.lookup(step.tool);
  if (!tool) {
    return h('div', { class: 'mins' }, section('Bilinmeyen araç', h('div', { class: 'mins__note' }, `“${step.tool}” bu sürümde yok. Adımı silin ya da aracı sağlayan eklentiyi yükleyin.`)));
  }
  const runner = ctx.processing.runner;
  const defaults = runner.defaults();
  // Fixed values and tool defaults: what the step's own controls and visibility read.
  const fixed = () => Object.fromEntries(tool.parameters.map((p) => [p.name, step.values[p.name]?.kind === 'value' ? (step.values[p.name] as { value: unknown }).value : defaultValue(p, defaults)]));
  const known = fixed();
  const env: FieldEnv = {
    ctx,
    describe: (name) => runner.describeInputs(tool, fixed())[name],
    previewExpression: (name) => runner.previewExpression(tool, fixed(), name),
    pickPoint: (name) => host.pickPoint(stepId, name),
  };

  const paramRow = (p: ParamDef) => {
    const src = step.values[p.name];
    const options = sourcesFor(model, stepId, p, host.lookup);
    const asInput = inputTypeFor(p);
    const items = (): MenuItem[] => [
      { label: 'Aracın varsayılanı', radio: true, checked: !src, run: () => host.change(() => setSource(model, stepId, p.name, null)) },
      { label: 'Sabit değer', radio: true, checked: src?.kind === 'value', run: () => host.change(() => setSource(model, stepId, p.name, { kind: 'value', value: src?.kind === 'value' ? src.value : defaultValue(p, defaults) })) },
      ...(options.length ? [{ kind: 'separator' } as MenuItem] : []),
      ...options.map(
        (o): MenuItem => ({
          label: o.label,
          detail: o.group === 'Girdi' ? 'Model girdisi' : o.group,
          radio: true,
          checked: JSON.stringify(src) === JSON.stringify(o.src),
          run: () => host.change(() => setSource(model, stepId, p.name, o.src)),
        }),
      ),
      ...(asInput
        ? [
            { kind: 'separator' } as MenuItem,
            {
              label: 'Yeni model girdisi yap',
              icon: 'modelNew',
              detail: 'Model çalıştırılırken bu değer sorulur',
              run: () =>
                host.change(() => {
                  const name = addInput(model, asInput, p.label, { x: (step.position?.x ?? 300) - 290, y: step.position?.y ?? 40 });
                  const created = model.inputs.find((i) => i.name === name)!;
                  const d = defaultValue(p, defaults);
                  if (asInput === 'features' && p.type === 'features') Object.assign(created, { kinds: p.kinds ? [...p.kinds] : undefined, default: d });
                  else if (asInput === 'number' && p.type === 'number') Object.assign(created, { default: d, min: p.min, max: p.max, integer: p.integer, unit: p.unit });
                  else if (asInput === 'string') Object.assign(created, { default: typeof d === 'string' ? d : '', allowEmpty: p.type === 'string' ? p.allowEmpty : undefined });
                  else if (d !== null && d !== undefined) Object.assign(created, { default: d });
                  setSource(model, stepId, p.name, { kind: 'input', name });
                }),
            } as MenuItem,
          ]
        : []),
    ];
    const dd = new Dropdown({ ariaLabel: `${p.label}: kaynak`, className: 'pfield__dropdown mins__source', items });
    dd.set(h('span', { class: 'dropdown__text' }, sourceText(host, src)));
    const control =
      src?.kind === 'value'
        ? paramControl(p, src.value, (v, rebuild) => host.change(() => setSource(model, stepId, p.name, { kind: 'value', value: v }), { rerender: rebuild !== false, key: `${stepId}.${p.name}` }), env)
        : null;
    return h(
      'div',
      { class: 'mins__param', 'data-linked': src && src.kind !== 'value' ? '' : null },
      h('div', { class: 'mins__label' }, p.label, p.optional ? h('span', { class: 'prow__opt' }, 'isteğe bağlı') : null),
      dd.el,
      control,
    );
  };

  const shown = tool.parameters.filter((p) => step.values[p.name] || isVisible(p, known));
  const main = shown.filter((p) => !p.advanced);
  const advanced = shown.filter((p) => p.advanced);
  const del = h('button', { class: 'btn btn--ghost mins__danger', type: 'button' }, icon('erase', 14), 'Adımı sil');
  del.addEventListener('click', () =>
    host.change(() => {
      removeStep(model, stepId);
      host.select(null);
    }),
  );

  return h(
    'div',
    { class: 'mins' },
    h('div', { class: 'mins__head' }, h('span', { class: 'mins__icon' }, icon(tool.icon ?? 'processing', 18)), h('div', null, h('div', { class: 'mins__kind' }, tool.label), h('div', { class: 'mins__lead' }, tool.description))),
    section(null, row('Başlık', textField({ label: 'Adım başlığı', value: step.caption ?? '', placeholder: tool.label, onChange: (v) => host.change(() => (step.caption = v.trim() || undefined), { rerender: false, key: `${stepId}.caption` }) }), 'Diyagramda ve iletilerde görünür.')),
    problemsSection(host, stepId),
    section('Parametreler', ...main.map(paramRow)),
    advanced.length ? h('details', { class: 'mins__advanced' }, h('summary', null, `Gelişmiş (${advanced.length})`), ...advanced.map(paramRow)) : null,
    section(null, del),
  );
}
