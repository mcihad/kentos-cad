import type { AppContext } from '../../../app/context';
import { ENTITY_KIND_LABEL, type EntityKind } from '../../../model/entities';
import { stepName, type ModelIssue, type ProcessingModel } from '../../../processing/model';
import { addOutput as addModelOutput, INPUT_TYPES, inputFromParam, inputTypeFor, removeInput, removeStep, setCaption, setSource, sourcesFor } from '../../../processing/modelEdit';
import { defaultValue, isVisible } from '../../../processing/parameters';
import type { FeaturesValue, ParamDef, ProcessingTool } from '../../../processing/types';
import { h } from '../../dom';
import { icon } from '../../icons';
import { segmented, textField, toggleSwitch } from '../../widgets/controls';
import { Dropdown } from '../../widgets/Dropdown';
import type { MenuItem } from '../../widgets/PopupMenu';
import { paramControl, type FieldEnv } from '../paramFields';
import { DESIGNER_TEXTS, sourceText, type NodeRef } from './designerPlan';

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
  /** Sahneden seç beside a step's choice that picks a point: the point and the choice's option, both fixed. */
  pickChoice(stepId: string, param: string): void;
  deleteModel(): void;
}

const KIND_CHOICES: readonly EntityKind[] = ['polygon', 'polyline', 'line', 'point', 'circle', 'arc', 'text'];
const T = DESIGNER_TEXTS.inspector;

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
    ariaLabel: T.model.category,
    className: 'pfield__dropdown',
    items: () => all.map((c): MenuItem => ({ label: c.label, icon: c.icon, radio: true, checked: c.id === model.category, run: () => host.change(() => (model.category = c.id)) })),
  });
  category.set(h('span', { class: 'dropdown__text' }, registry.category(model.category)?.label ?? model.category));

  const outputs = model.outputs.map((o) => {
    const step = model.steps.find((s) => s.id === o.from.step);
    const out = step && host.lookup(step.tool)?.outputs?.find((x) => x.name === o.from.output);
    const remove = h('button', { class: 'ibtn', type: 'button', 'aria-label': T.model.removeOutput(o.label) }, icon('close', 14));
    remove.addEventListener('click', () => host.change(() => (model.outputs = model.outputs.filter((x) => x !== o))));
    return h('div', { class: 'mins__item' }, h('span', { class: 'mins__item-main' }, h('b', null, o.label), h('span', null, step ? T.model.output(stepName(step, host.lookup), out?.label ?? o.from.output) : T.model.noStep)), remove);
  });
  const candidates = model.steps.flatMap((s) =>
    (host.lookup(s.tool)?.outputs ?? []).filter((o) => !model.outputs.some((x) => x.from.step === s.id && x.from.output === o.name)).map((o) => ({ step: s, out: o })),
  );
  const addOutput = new Dropdown({
    ariaLabel: T.model.addOutput,
    className: 'pfield__dropdown',
    items: () =>
      candidates.length
        ? candidates.map(({ step, out }): MenuItem => ({
            label: out.label,
            detail: stepName(step, host.lookup),
            run: () =>
              host.change(() => void addModelOutput(model, step.id, out.name, host.lookup)),
          }))
        : [{ label: T.model.noOutput, disabled: true }],
  });
  addOutput.set(h('span', { class: 'dropdown__text' }, T.model.addOutput));

  const del = host.saved && !host.builtinCopy ? h('button', { class: 'btn btn--ghost mins__danger', type: 'button' }, icon('erase', 14), T.model.remove) : null;
  del?.addEventListener('click', () => host.deleteModel());

  return h(
    'div',
    { class: 'mins' },
    h('div', { class: 'mins__head' }, h('span', { class: 'mins__icon' }, icon('processing', 18)), h('div', null, h('div', { class: 'mins__kind' }, T.model.kind), h('div', { class: 'mins__lead' }, T.model.lead))),
    section(
      null,
      row(T.model.name, textField({ label: T.model.nameAria, value: model.label, onChange: (v) => host.change(() => (model.label = v), { rerender: false, key: 'label' }) })),
      row(T.model.category, category.el),
      row(T.model.description, textField({ label: T.model.descriptionAria, value: model.description, placeholder: T.model.descriptionHint, onChange: (v) => host.change(() => (model.description = v), { rerender: false, key: 'description' }) })),
    ),
    section(T.model.outputs, ...outputs, addOutput.el),
    problemsSection(host, null),
    del ? section(null, del) : null,
  );
}

function problemsSection(host: InspectorHost, stepId: string | null): HTMLElement | null {
  const list = host.problems.filter((p) => stepId === null || p.step === stepId);
  if (!list.length) return stepId === null ? section(null, h('div', { class: 'mins__ok' }, icon('check', 14), host.model.steps.length ? T.problems.ready : T.problems.start)) : null;
  return section(
    stepId ? T.problems.title : T.problems.titleCount(list.length),
    ...list.map((p) => {
      const step = p.step ? host.model.steps.find((s) => s.id === p.step) : null;
      const b = h('button', { class: 'mins__problem', type: 'button' }, icon('warning', 14), h('span', null, step && !stepId ? T.problems.ofStep(stepName(step, host.lookup), p.message) : p.message));
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
        T.input.default,
        segmented({
          label: T.input.scopeAria,
          options: [
            { value: 'selection', label: T.input.scopes.selection },
            { value: 'visible', label: T.input.scopes.visible },
            { value: 'all', label: T.input.scopes.all },
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
    specific.push(row(T.input.kinds, h('div', { class: 'pfield__chips' }, chips), kinds.size ? null : T.input.kindsNote));
  } else if (def.type === 'number') {
    const num = (label: string, key: 'default' | 'min' | 'max', value: number | undefined, optional: boolean) => {
      const input = h('input', { class: 'field pfield__num num', value: value === undefined ? '' : String(value), inputmode: 'decimal', 'aria-label': label, placeholder: optional ? T.input.none : '' });
      input.addEventListener('input', () => {
        const t = input.value.trim().replace(',', '.');
        const n = Number(t);
        if (t === '' && optional) set(key, undefined, false);
        else if (t !== '' && Number.isFinite(n)) set(key, n, false);
      });
      return row(label, input);
    };
    specific.push(
      num(T.input.default, 'default', typeof d.default === 'number' ? d.default : 0, false),
      num(T.input.min, 'min', def.min, true),
      num(T.input.max, 'max', def.max, true),
      row(T.input.integer, toggleSwitch({ label: T.input.integer, checked: !!def.integer, onChange: (v) => set('integer', v || undefined) })),
    );
  } else if (def.type === 'string') {
    specific.push(
      row(T.input.default, textField({ label: T.input.textDefault, value: String(d.default ?? ''), onChange: (v) => set('default', v, false) })),
      row(T.input.allowEmpty, toggleSwitch({ label: T.input.allowEmpty, checked: !!def.allowEmpty, onChange: (v) => set('allowEmpty', v || undefined) })),
    );
  } else if (def.type === 'boolean') {
    specific.push(row(T.input.default, toggleSwitch({ label: T.input.default, checked: !!d.default, onChange: (v) => set('default', v) })));
  } else if (def.type === 'layer') {
    const dv = d.default as { newName?: string } | undefined;
    specific.push(row(T.input.newLayer, textField({ label: T.input.newLayerAria, value: dv?.newName ?? '', onChange: (v) => set('default', { newName: v }, false) }), T.input.newLayerNote));
  } else if (def.type === 'point') {
    specific.push(h('div', { class: 'mins__note' }, T.input.pointNote));
  }

  const del = h('button', { class: 'btn btn--ghost mins__danger', type: 'button' }, icon('erase', 14), T.input.remove);
  del.addEventListener('click', () =>
    host.change(() => {
      removeInput(host.model, def.name);
      host.select(null);
    }),
  );

  return h(
    'div',
    { class: 'mins' },
    h('div', { class: 'mins__head' }, h('span', { class: 'mins__icon mins__icon--input' }, icon(type?.icon ?? 'processing', 18)), h('div', null, h('div', { class: 'mins__kind' }, T.input.kind(type?.label ?? def.type)), h('div', { class: 'mins__lead' }, T.input.lead))),
    section(
      null,
      row(T.input.label, textField({ label: T.input.labelAria, value: def.label, onChange: (v) => set('label', v, false) }), T.input.variable(def.name)),
      row(T.input.description, textField({ label: T.input.descriptionAria, value: d.description ?? '', placeholder: T.input.descriptionHint, onChange: (v) => set('description', v || undefined, false) })),
      row(T.input.optional, toggleSwitch({ label: T.input.optional, checked: !!d.optional, onChange: (v) => set('optional', v || undefined) })),
      ...specific,
    ),
    section(
      T.input.users,
      ...(users.length
        ? users.map((s) => {
            const b = h('button', { class: 'mins__link', type: 'button' }, icon(host.lookup(s.tool)?.icon ?? 'processing', 14), stepName(s, host.lookup));
            b.addEventListener('click', () => host.select({ kind: 'step', id: s.id }));
            return b;
          })
        : [h('div', { class: 'mins__note' }, T.input.noUsers)]),
    ),
    section(null, del),
  );
}

// ── A step ─────────────────────────────────────────────────────────────

function stepInspector(host: InspectorHost, stepId: string): HTMLElement {
  const { model, ctx } = host;
  const step = model.steps.find((s) => s.id === stepId)!;
  const tool = host.lookup(step.tool);
  if (!tool) {
    return h('div', { class: 'mins' }, section(T.step.unknown, h('div', { class: 'mins__note' }, T.step.unknownNote(step.tool))));
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
    builderObjects: (name) => runner.builderObjects(tool, fixed(), name),
    pickPoint: (name) => host.pickPoint(stepId, name),
    pickChoice: (name) => host.pickChoice(stepId, name),
  };

  const paramRow = (p: ParamDef) => {
    const src = step.values[p.name];
    const options = sourcesFor(model, stepId, p, host.lookup);
    const asInput = inputTypeFor(p);
    const items = (): MenuItem[] => [
      { label: T.step.toolDefault, radio: true, checked: !src, run: () => host.change(() => setSource(model, stepId, p.name, null)) },
      { label: T.step.fixed, radio: true, checked: src?.kind === 'value', run: () => host.change(() => setSource(model, stepId, p.name, { kind: 'value', value: src?.kind === 'value' ? src.value : defaultValue(p, defaults) })) },
      ...(options.length ? [{ kind: 'separator' } as MenuItem] : []),
      ...options.map(
        (o): MenuItem => ({
          label: o.label,
          detail: o.group === 'Girdi' ? T.step.modelInput : o.group,
          radio: true,
          checked: JSON.stringify(src) === JSON.stringify(o.src),
          run: () => host.change(() => setSource(model, stepId, p.name, o.src)),
        }),
      ),
      ...(asInput
        ? [
            { kind: 'separator' } as MenuItem,
            {
              label: T.step.asInput,
              icon: 'modelNew',
              detail: T.step.asInputNote,
              run: () => host.change(() => void inputFromParam(model, stepId, p, defaults)),
            } as MenuItem,
          ]
        : []),
    ];
    const dd = new Dropdown({ ariaLabel: T.step.sourceAria(p.label), className: 'pfield__dropdown mins__source', items });
    dd.set(h('span', { class: 'dropdown__text' }, sourceText(model, src, host.lookup)));
    const control =
      src?.kind === 'value'
        ? paramControl(p, src.value, (v, rebuild) => host.change(() => setSource(model, stepId, p.name, { kind: 'value', value: v }), { rerender: rebuild !== false, key: `${stepId}.${p.name}` }), env)
        : null;
    return h(
      'div',
      { class: 'mins__param', 'data-linked': src && src.kind !== 'value' ? '' : null },
      h('div', { class: 'mins__label' }, p.label, p.optional ? h('span', { class: 'prow__opt' }, T.step.optional) : null),
      dd.el,
      control,
    );
  };

  const shown = tool.parameters.filter((p) => step.values[p.name] || isVisible(p, known));
  const main = shown.filter((p) => !p.advanced);
  const advanced = shown.filter((p) => p.advanced);
  const del = h('button', { class: 'btn btn--ghost mins__danger', type: 'button' }, icon('erase', 14), T.step.remove);
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
    section(null, row(T.step.caption, textField({ label: T.step.captionAria, value: step.caption ?? '', placeholder: tool.label, onChange: (v) => host.change(() => setCaption(model, stepId, v), { rerender: false, key: `${stepId}.caption` }) }), T.step.captionNote)),
    problemsSection(host, stepId),
    section(T.step.params, ...main.map(paramRow)),
    advanced.length ? h('details', { class: 'mins__advanced' }, h('summary', null, T.step.advanced(advanced.length)), ...advanced.map(paramRow)) : null,
    section(null, del),
  );
}
