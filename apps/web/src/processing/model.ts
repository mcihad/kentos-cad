import { isVisible } from './parameters';
import type { ParamDef, ParamType, ProcessingTool } from './types';

/**
 * Models (QGIS "Model Designer" gibi): tools wired into a flow diagram.
 * This file fixes the data shape now so the core stays compatible; the
 * diagram editor and the model runner come later (docs/PROCESSING.md).
 *
 * A step runs one tool. Each of its parameters takes a fixed value, one
 * of the model's own inputs, or an output of an earlier step — a features
 * output becomes a { scope: 'ids' } value, so steps chain through the
 * objects they create.
 */

export type ValueSource = { kind: 'value'; value: unknown } | { kind: 'input'; name: string } | { kind: 'output'; step: string; output: string };

export interface ModelStep {
  /** Unique within the model. */
  id: string;
  /** Processing tool id. */
  tool: string;
  values: Record<string, ValueSource>;
  /** Box position in the diagram (px). */
  position?: { x: number; y: number };
  /** Short caption in the diagram; the tool name when absent. */
  caption?: string;
}

export interface ProcessingModel {
  id: string;
  label: string;
  category: string;
  description: string;
  /** What the user fills in when running the model (same definitions as tool parameters). */
  inputs: ParamDef[];
  steps: ModelStep[];
  /** Model outputs, taken from step outputs. */
  outputs: { name: string; label: string; from: { step: string; output: string } }[];
  /** Positions of the input boxes in the diagram (px), by input name. */
  inputPositions?: Record<string, { x: number; y: number }>;
}

export type ModelIssue = { step?: string; message: string };

/** Kinds of value a model input or a step output can feed. */
export type SourceType = ParamType | 'string' | 'number' | 'features';

/**
 * Whether a value of type `from` (a model input or a step output) can feed
 * a parameter of type `to`: same type, a number or text where text is
 * written, and text into an expression or a field name.
 */
export function canFeed(from: SourceType, to: ParamType): boolean {
  if (from === to) return true;
  if (to === 'string') return from === 'number' || from === 'enum';
  if (to === 'expression' || to === 'field') return from === 'string';
  return false;
}

/** Required and with nothing to fall back on: a point, an expression, a field or a text that may not be empty. */
function needsSource(p: ParamDef): boolean {
  if (p.optional || 'default' in p) return false;
  return p.type === 'point' || p.type === 'expression' || p.type === 'field' || (p.type === 'string' && !p.allowEmpty);
}

/** How a step is named in messages and in the diagram. */
export const stepName = (step: ModelStep, lookup: (id: string) => ProcessingTool | undefined) => step.caption || lookup(step.tool)?.label || step.tool;

/** Problems that stop a model from running: unknown tools, parameters, inputs, outputs, cycles. */
export function checkModel(model: ProcessingModel, lookup: (id: string) => ProcessingTool | undefined): ModelIssue[] {
  const issues: ModelIssue[] = [];
  const steps = new Map(model.steps.map((s) => [s.id, s]));
  if (steps.size !== model.steps.length) issues.push({ message: 'Adım kimlikleri benzersiz olmalı.' });
  const inputs = new Set(model.inputs.map((i) => i.name));
  for (const s of model.steps) {
    const tool = lookup(s.tool);
    if (!tool) {
      issues.push({ step: s.id, message: `Bilinmeyen işlem aracı: ${s.tool}` });
      continue;
    }
    // What is known before running: fixed values and fixed defaults (for visibleWhen).
    const known = Object.fromEntries(tool.parameters.map((p) => {
      const src = s.values[p.name];
      const d = (p as { default?: unknown }).default;
      return [p.name, src?.kind === 'value' ? src.value : typeof d === 'function' ? undefined : d];
    }));
    for (const p of tool.parameters) {
      const src = s.values[p.name];
      if (!src) {
        if (needsSource(p) && isVisible(p, known)) issues.push({ step: s.id, message: `“${p.label}” için bir değer ya da bağlantı gerekiyor.` });
        continue;
      }
      if (src.kind === 'input') {
        const input = model.inputs.find((i) => i.name === src.name);
        if (!input || !inputs.has(src.name)) issues.push({ step: s.id, message: `“${p.label}” olmayan bir model girdisine bağlı: ${src.name}` });
        else if (!canFeed(input.type, p.type)) issues.push({ step: s.id, message: `“${p.label}” “${input.label}” girdisinden beslenemez: türler uymuyor.` });
      }
      if (src.kind === 'output') {
        const from = steps.get(src.step);
        const fromTool = from && lookup(from.tool);
        const out = fromTool?.outputs?.find((o) => o.name === src.output);
        if (!out) issues.push({ step: s.id, message: `“${p.label}” olmayan bir çıktıya bağlı: ${src.step}.${src.output}` });
        else if (!canFeed(out.type, p.type)) issues.push({ step: s.id, message: `“${p.label}” “${out.label}” çıktısından beslenemez: türler uymuyor.` });
      }
    }
  }
  if (!('error' in orderSteps(model))) return issues;
  return [...issues, { message: (orderSteps(model) as { error: string }).error }];
}

/** Steps in an order where every step comes after the steps it reads from (Kahn), or the cycle. */
export function orderSteps(model: ProcessingModel): string[] | { error: string } {
  const deps = new Map(model.steps.map((s) => [s.id, new Set(Object.values(s.values).flatMap((v) => (v.kind === 'output' ? [v.step] : [])))]));
  const order: string[] = [];
  const ready = model.steps.filter((s) => !deps.get(s.id)!.size).map((s) => s.id);
  while (ready.length) {
    const id = ready.shift()!;
    order.push(id);
    for (const [other, d] of deps) {
      if (!d.delete(id)) continue;
      if (!d.size) ready.push(other);
    }
  }
  if (order.length === model.steps.length) return order;
  const stuck = model.steps.filter((s) => !order.includes(s.id)).map((s) => s.id);
  return { error: `Modelde döngü var: ${stuck.join(' → ')} birbirini bekliyor.` };
}
