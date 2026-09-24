import { foldTurkish } from '../core/text';
import { canFeed, orderSteps, stepName, type ModelStep, type ProcessingModel, type ValueSource } from './model';
import type { ParamDef, ProcessingTool } from './types';

/**
 * Editing a model draft (the model designer's operations), kept pure so it
 * is tested without the UI. Every function changes the draft in place;
 * the designer snapshots it before each change for its own undo.
 */

type Lookup = (id: string) => ProcessingTool | undefined;

/** Kinds of model input the designer offers (the parameter types a user can fill in). */
export type ModelInputType = 'features' | 'number' | 'string' | 'boolean' | 'layer' | 'point';

export const INPUT_TYPES: readonly { type: ModelInputType; label: string; icon: string; description: string }[] = [
  { type: 'features', label: 'Nesneler', icon: 'select', description: 'Seçili, görünen, bütün nesneler ya da bir katman' },
  { type: 'number', label: 'Sayı', icon: 'units', description: 'Mesafe, adet, ondalık basamak …' },
  { type: 'string', label: 'Metin', icon: 'text', description: 'Önek, alan adı, ifade …' },
  { type: 'boolean', label: 'Evet / hayır', icon: 'check', description: 'Açık ya da kapalı bir seçenek' },
  { type: 'layer', label: 'Katman', icon: 'layers', description: 'Sonuçların yazılacağı katman' },
  { type: 'point', label: 'Nokta', icon: 'point', description: 'Haritada gösterilen bir nokta' },
];

/** Identifier from a Turkish label: "Nokta öneki" → "noktaOneki". */
export function slug(label: string): string {
  const words = foldTurkish(label)
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
  const s = words.map((w, i) => (i ? w[0].toUpperCase() + w.slice(1) : w)).join('');
  return /^[a-z]/.test(s) ? s : `g${s}`;
}

const unique = (base: string, taken: (s: string) => boolean) => {
  let name = base;
  for (let k = 2; taken(name); k++) name = `${base}${k}`;
  return name;
};

/** A new model id: time plus a random tail, so two models made in the same millisecond differ. */
const modelId = () => `m-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 7)}`;

export function newModel(): ProcessingModel {
  return { id: modelId(), label: 'Yeni model', category: 'points', description: '', inputs: [], steps: [], outputs: [], inputPositions: {} };
}

/** Adds an input of a type; returns its name. */
export function addInput(model: ProcessingModel, type: ModelInputType, label: string, at?: { x: number; y: number }): string {
  const name = unique(slug(label), (n) => model.inputs.some((i) => i.name === n));
  const def: ParamDef =
    type === 'features'
      ? { type, name, label, default: { scope: 'selection' } }
      : type === 'number'
        ? { type, name, label, default: 0 }
        : type === 'string'
          ? { type, name, label, default: '', allowEmpty: true }
          : type === 'boolean'
            ? { type, name, label, default: false }
            : type === 'layer'
              ? { type, name, label, default: { newName: label } }
              : { type, name, label };
  model.inputs.push(def);
  model.inputPositions = { ...model.inputPositions, [name]: at ?? { x: 40, y: 40 + (model.inputs.length - 1) * 90 } };
  return name;
}

/** Removes an input; steps that read it fall back to the tool's default. */
export function removeInput(model: ProcessingModel, name: string): void {
  model.inputs = model.inputs.filter((i) => i.name !== name);
  if (model.inputPositions) delete model.inputPositions[name];
  for (const s of model.steps) for (const [p, src] of Object.entries(s.values)) if (src.kind === 'input' && src.name === name) delete s.values[p];
}

/**
 * Adds a step running a tool; returns its id. When `from` is given (the
 * node the user had selected) the new step's first input it can take is
 * connected to it, so adding tools one after another builds a chain.
 */
export function addStep(model: ProcessingModel, toolId: string, lookup: Lookup, at?: { x: number; y: number }, from?: { kind: 'input'; name: string } | { kind: 'step'; id: string }): string {
  const tool = lookup(toolId);
  const id = unique(slug(tool?.label ?? toolId), (n) => model.steps.some((s) => s.id === n));
  const step: ModelStep = { id, tool: toolId, values: {}, position: at ?? nextFreeSpot(model) };
  model.steps.push(step);
  if (tool && from) {
    for (const p of tool.parameters) {
      const src = sourcesFor(model, id, p, lookup).find((o) => (from.kind === 'input' ? o.src.kind === 'input' && o.src.name === from.name : o.src.kind === 'output' && o.src.step === from.id));
      if (src) {
        step.values[p.name] = src.src;
        break;
      }
    }
  }
  return id;
}

function nextFreeSpot(model: ProcessingModel): { x: number; y: number } {
  const xs = model.steps.map((s) => s.position?.x ?? 0);
  const x = xs.length ? Math.max(...xs) + 280 : 300;
  return { x, y: 60 };
}

/** Removes a step, the connections that read its outputs and the model outputs taken from it. */
export function removeStep(model: ProcessingModel, id: string): void {
  model.steps = model.steps.filter((s) => s.id !== id);
  for (const s of model.steps) for (const [p, src] of Object.entries(s.values)) if (src.kind === 'output' && src.step === id) delete s.values[p];
  model.outputs = model.outputs.filter((o) => o.from.step !== id);
}

/** Sets where a step parameter gets its value; null goes back to the tool's default. */
export function setSource(model: ProcessingModel, stepId: string, param: string, src: ValueSource | null): void {
  const step = model.steps.find((s) => s.id === stepId);
  if (!step) return;
  if (src) step.values[param] = src;
  else delete step.values[param];
}

export interface SourceOption {
  src: ValueSource;
  label: string;
  /** Where it comes from, for menus: "Girdi" or the step name. */
  group: string;
}

/** Inputs and earlier outputs that can feed a parameter (never the step itself, never a cycle). */
export function sourcesFor(model: ProcessingModel, stepId: string, param: ParamDef, lookup: Lookup): SourceOption[] {
  const out: SourceOption[] = [];
  for (const i of model.inputs) if (canFeed(i.type, param.type)) out.push({ src: { kind: 'input', name: i.name }, label: i.label, group: 'Girdi' });
  const downstream = dependents(model, stepId);
  for (const s of model.steps) {
    if (s.id === stepId || downstream.has(s.id)) continue;
    for (const o of lookup(s.tool)?.outputs ?? []) if (canFeed(o.type, param.type)) out.push({ src: { kind: 'output', step: s.id, output: o.name }, label: o.label, group: stepName(s, lookup) });
  }
  return out;
}

/** Steps that read (directly or not) from a step: they cannot feed it. */
function dependents(model: ProcessingModel, stepId: string): Set<string> {
  const found = new Set<string>();
  const visit = (id: string) => {
    for (const s of model.steps)
      if (!found.has(s.id) && Object.values(s.values).some((v) => v.kind === 'output' && v.step === id)) {
        found.add(s.id);
        visit(s.id);
      }
  };
  visit(stepId);
  return found;
}

export interface ModelEdge {
  from: { kind: 'input'; name: string } | { kind: 'step'; id: string };
  to: string;
  /** Parameters of the target step fed by this source. */
  params: string[];
}

/** One edge per (source, target step) pair, for the diagram. */
export function edgesOf(model: ProcessingModel): ModelEdge[] {
  const map = new Map<string, ModelEdge>();
  for (const s of model.steps)
    for (const [p, src] of Object.entries(s.values)) {
      if (src.kind === 'value') continue;
      const from = src.kind === 'input' ? ({ kind: 'input', name: src.name } as const) : ({ kind: 'step', id: src.step } as const);
      const key = `${from.kind}:${from.kind === 'input' ? from.name : from.id}→${s.id}`;
      const e: ModelEdge = map.get(key) ?? { from, to: s.id, params: [] };
      e.params.push(p);
      map.set(key, e);
    }
  return [...map.values()];
}

/** Lays the diagram out in columns: inputs, then each step one column after what it reads. */
export function autoLayout(model: ProcessingModel): void {
  const depth = new Map<string, number>();
  const ids = orderSteps(model);
  const order = Array.isArray(ids) ? ids : model.steps.map((s) => s.id);
  for (const id of order) {
    const s = model.steps.find((x) => x.id === id)!;
    const deps = Object.values(s.values).flatMap((v) => (v.kind === 'output' ? [depth.get(v.step) ?? 0] : []));
    depth.set(id, deps.length ? Math.max(...deps) + 1 : 1);
  }
  const rows = new Map<number, number>();
  const place = (col: number) => {
    const row = rows.get(col) ?? 0;
    rows.set(col, row + 1);
    return { x: 40 + col * 290, y: 40 + row * 100 };
  };
  model.inputPositions = Object.fromEntries(model.inputs.map((i) => [i.name, place(0)]));
  for (const id of order) {
    const s = model.steps.find((x) => x.id === id)!;
    s.position = place(depth.get(id) ?? 1);
  }
}

/** A copy of a model under a new id (to change a built-in one). */
export function copyModel(model: ProcessingModel): ProcessingModel {
  const copy = JSON.parse(JSON.stringify(model)) as ProcessingModel;
  return { ...copy, id: modelId(), label: `${model.label} (kopya)` };
}
