import { checkModel, orderSteps, stepName, type ModelStep, type ProcessingModel, type ValueSource } from './model';
import { defaultValues } from './parameters';
import type { ProcessingRunner, RunOptions, RunOutcome } from './runner';
import type { ProcessingTool, RunResult } from './types';

/**
 * Runs a model: its steps in dependency order, each one a normal tool run
 * whose values come from fixed values, the model's inputs or earlier
 * steps' outputs (a features output reaches the next step as the ids of
 * the objects it made or chose). The whole model is one undo step; if a
 * step fails or is stopped, what the earlier steps did is taken back.
 */

export type Lookup = (id: string) => ProcessingTool | undefined;

/** Prefix of model ids in history and commands ("model:m-1a2b"). */
export const MODEL_PREFIX = 'model:';

/** A model as the dialog sees it: its inputs are the parameters. */
export function modelAsTool(model: ProcessingModel, lookup: Lookup): ProcessingTool {
  const steps = orderSteps(model);
  const order = Array.isArray(steps) ? steps.map((id) => model.steps.find((s) => s.id === id)!) : model.steps;
  return {
    id: `${MODEL_PREFIX}${model.id}`,
    label: model.label,
    category: model.category,
    description: model.description || `${model.steps.length} adımlı model.`,
    help: order.map((s, i) => `${i + 1}. ${stepName(s, lookup)}`).join('\n\n'),
    icon: 'processing',
    parameters: model.inputs,
    targets: ['client'],
    run: () => {
      throw new Error('Modeller adım adım çalıştırılır (runModel).');
    },
  };
}

/** The value a step parameter gets from its source. */
function sourceValue(src: ValueSource, inputs: Record<string, unknown>, outputs: Map<string, Record<string, unknown>>, lookup: Lookup, steps: Map<string, ModelStep>): unknown {
  if (src.kind === 'value') return src.value;
  if (src.kind === 'input') return inputs[src.name];
  const out = outputs.get(src.step)?.[src.output];
  const def = lookup(steps.get(src.step)?.tool ?? '')?.outputs?.find((o) => o.name === src.output);
  return def?.type === 'features' ? { scope: 'ids', ids: Array.isArray(out) ? out : [] } : out;
}

/** A step's outputs: features as the ids it returned (selected, changed) or else the ones it added. */
function stepOutputs(tool: ProcessingTool, result: RunResult, added: readonly number[]): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const o of tool.outputs ?? []) {
    const v = result.outputs?.[o.name];
    out[o.name] = o.type === 'features' ? (Array.isArray(v) ? v : [...added]) : v;
  }
  return out;
}

export async function runModel(model: ProcessingModel, inputs: Record<string, unknown>, runner: ProcessingRunner, lookup: Lookup, opts: Pick<RunOptions, 'log' | 'target'> = {}): Promise<RunOutcome> {
  const asTool = modelAsTool(model, lookup);
  const inputIssues = runner.validate(asTool, inputs);
  if (inputIssues.length) return { status: 'invalid', issues: inputIssues };
  const started = Date.now();
  const copy = JSON.parse(JSON.stringify(inputs)) as Record<string, unknown>;
  const steps = new Map(model.steps.map((s) => [s.id, s]));
  const record = (status: 'ok' | 'error' | 'canceled', summary: string, added: number[] = [], touched: number[] = []) =>
    runner.addRecord({ toolId: asTool.id, label: model.label, values: copy, started, ms: Date.now() - started, status, summary, added, touched });

  const problems = checkModel(model, lookup);
  if (problems.length) {
    const p = problems[0];
    const where = p.step && steps.get(p.step) ? `“${stepName(steps.get(p.step)!, lookup)}” adımı: ` : '';
    const message = `Model çalıştırılamaz. ${where}${p.message}${problems.length > 1 ? ` (${problems.length - 1} sorun daha var; modeli düzenleyin)` : ''}`;
    return { status: 'error', message, record: record('error', message) };
  }
  const order = orderSteps(model) as string[];
  if (!order.length) {
    const message = 'Modelde adım yok. Model tasarımcısında bir araç ekleyin.';
    return { status: 'error', message, record: record('error', message) };
  }

  const group = runner.beginGroup(model.label);
  const outputs = new Map<string, Record<string, unknown>>();
  const added: number[] = [];
  const touched = new Set<number>();
  const summaries: string[] = [];
  let edited = false;
  let select: readonly number[] | undefined;
  for (let i = 0; i < order.length; i++) {
    const step = steps.get(order[i])!;
    const tool = lookup(step.tool)!;
    const name = `${i + 1}. adım (${stepName(step, lookup)})`;
    const values = defaultValues(tool, runner.defaults());
    for (const [param, src] of Object.entries(step.values)) values[param] = sourceValue(src, inputs, outputs, lookup, steps);
    // The chosen place where the step can go there; "auto" otherwise.
    const target = opts.target && opts.target !== 'auto' && runner.executorFor(tool, opts.target) ? opts.target : 'auto';
    runner.running.set({ toolId: asTool.id, fraction: i / order.length, label: `${name} çalışıyor` });
    const out = await runner.run(tool, values, { log: opts.log, target, silent: true });
    if (out.status !== 'ok') {
      group.cancel();
      const why = out.status === 'invalid' ? out.issues[0]?.message : out.message;
      const status = out.status === 'canceled' ? 'canceled' : 'error';
      const message = status === 'canceled' ? 'Model durduruldu; çizim değişmedi.' : `${name} çalışmadı: ${why} Önceki adımların sonuçları geri alındı.`;
      return { status, message, record: record(status, message) };
    }
    outputs.set(step.id, stepOutputs(tool, out.result, out.added));
    added.push(...out.added);
    for (const id of out.touched) touched.add(id);
    edited ||= out.edited;
    if (out.result.select) select = out.result.select;
    summaries.push(out.record.summary);
  }
  group.end();
  runner.running.set(null);
  const modelOutputs = Object.fromEntries(model.outputs.map((o) => [o.name, outputs.get(o.from.step)?.[o.from.output]]));
  const summary = `${order.length} adım çalıştı. ${summaries.join(' ')}`;
  const result: RunResult = { outputs: modelOutputs, summary, select };
  const t = [...touched];
  return { status: 'ok', result, added, touched: t, edited, record: record('ok', summary, added, t) };
}
