import { Formatter } from '../../app/format';
import { CadDocument } from '../../model/document';
import type { EntityKind } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { LayerStore } from '../../model/layers';
import { readSnapshot } from '../../model/snapshot';
import { BUILTIN_TOOLS } from '../../processing/builtin';
import { BUILTIN_MODELS } from '../../processing/builtin/models';
import { clientExecutor, type Executor } from '../../processing/job';
import { modelAsTool, MODEL_PREFIX, runModel } from '../../processing/modelRunner';
import { ProcessingRegistry } from '../../processing/registry';
import { ProcessingRunner, type RunOutcome, type TargetChoice } from '../../processing/runner';
import type { ExecutionTarget, FeaturesValue, ProcessingTool } from '../../processing/types';
import { handleJob } from '../../processing/worker/handleJob';
import type { WorkerRequest } from '../../processing/worker/protocol';
import { workerExecutor, type WorkerLike } from '../../processing/worker/workerExecutor';
import {
  attempted,
  chosenTarget,
  dialogForm,
  dialogView,
  edited,
  effectiveChoice,
  finished,
  hostEnv,
  modelSteps,
  startState,
  picked,
  resetState,
  started,
  toggledAdvanced,
  undone,
  type DialogForm,
  type DialogState,
  type DialogView,
} from './dialogPlan';
import { DIALOG_TEXTS } from './dialogTexts';
import { numberOfText, toggleKind, withLayer, withScope } from './fieldPlan';

/**
 * Plays the processing dialog's sessions (fixtures/processing/v1/
 * dialog.json, format in fixtures/processing/README.md): the dialog's
 * state (dialogPlan) on a drawing and a runner, as ToolDialog keeps it,
 * with what it shows after each step. Used by the checker
 * (dialogFixture.test.ts) and the recorder
 * (scripts/fixtures/record-processing-dialog.test.ts); the app does not
 * load it.
 */

type Values = Record<string, unknown>;

/** One thing the user does in the dialog. */
export type SessionAction =
  /** A choice: an option, a switch, a list item (choosing what is already chosen changes nothing). */
  | { choose: { name: string; value: unknown } }
  /** Typing into a field: a number field reads the text as a number, a layer field as the new layer's name. */
  | { type: { name: string; text: string } }
  /** A scope on a features field's segmented control. */
  | { scope: { name: string; scope: 'selection' | 'visible' | 'all' | 'layer' } }
  /** A layer from the Katman scope's list. */
  | { scopeLayer: { name: string; layerId: string } }
  /** A kind chip clicked. */
  | { kind: { name: string; kind: EntityKind } }
  | { toggleAdvanced: true }
  | { target: TargetChoice }
  | { reset: true }
  /** Çalıştır: the run goes to the end before the next step. */
  | { run: true }
  /** Geri al on the status line after a run. */
  | { undo: true }
  /** A point shown on the map (null: the user gave up); the dialog comes back as it was. */
  | { pick: { name: string; point: Vec2 | null } };

export interface SessionSpec {
  id: string;
  title: string;
  open: { tool: string } | { model: string };
  /** The selection before the dialog opens (object ids). */
  selection?: number[];
  /** The visible area [minX, minY, maxX, maxY] for the "visible" scope; none by default. */
  view?: [number, number, number, number];
  /** Places this host can run tools: the desktop only "client" today, the web "client" and "worker". */
  available: ExecutionTarget[];
  /** Values the dialog is opened with (history's Yeniden aç). */
  given?: Values;
  /** The tool's last run's values, used when none are given. */
  last?: Values;
  /** The stored place to run on; Otomatik by default. */
  choice?: TargetChoice;
  steps: { do: SessionAction }[];
}

/** Sample arguments of the texts that are made from a value, as the file writes them. */
const TEXT_SAMPLES: Record<string, unknown> = {
  fixFields: 2,
  autoNow: 'worker',
  kindsNote: ['polygon', 'polyline'],
  newItem: 'Köşe noktaları',
  count: 3,
  has: 3,
  chipTitle: 3,
  more: 2,
};

/** The dialog's texts as the file writes them: a text made from a value as `{ sample, text }`. */
export function fixtureTexts(): unknown {
  const walk = (o: object): unknown =>
    Object.fromEntries(
      Object.entries(o).map(([k, v]) => {
        if (typeof v === 'function') {
          if (!(k in TEXT_SAMPLES)) throw new Error(`Metin örneği yok: ${k}`);
          return [k, { sample: TEXT_SAMPLES[k], text: (v as (x: unknown) => string)(TEXT_SAMPLES[k]) }];
        }
        return [k, v && typeof v === 'object' ? walk(v as object) : v];
      }),
    );
  return walk(DIALOG_TEXTS);
}

const TOOLS = new Map(BUILTIN_TOOLS.map((t) => [t.id, t]));
const MODELS = new Map(BUILTIN_MODELS.map((m) => [m.id, m]));
const lookup = (id: string) => TOOLS.get(id);

/** The registry the forms read categories from. */
export function builtinRegistry(): ProcessingRegistry {
  const r = new ProcessingRegistry();
  for (const t of BUILTIN_TOOLS) r.register(t);
  return r;
}

/** Every built-in tool's and model's form, by the id the dialog is opened with. */
export function builtinForms(): Record<string, DialogForm> {
  const registry = builtinRegistry();
  const host = { categoryPath: (id: string) => registry.categoryPath(id), categoryIcon: (id: string) => registry.category(id)?.icon };
  const out: Record<string, DialogForm> = {};
  for (const t of BUILTIN_TOOLS) out[t.id] = dialogForm(t, host);
  for (const m of BUILTIN_MODELS) out[`${MODEL_PREFIX}${m.id}`] = dialogForm(modelAsTool(m, lookup), host, { steps: modelSteps(m, lookup), builtin: true });
  return out;
}

/** A Worker stand-in: messages are structured-cloned both ways, as the browser does. */
function fakeWorker(): WorkerLike {
  const w: WorkerLike & { terminated: boolean } = {
    onmessage: null,
    onerror: null,
    terminated: false,
    postMessage(m) {
      const copy = structuredClone(m) as WorkerRequest;
      queueMicrotask(() => void handleJob(copy, (r) => !w.terminated && w.onmessage?.({ data: structuredClone(r) }), lookup));
    },
    terminate() {
      w.terminated = true;
    },
  };
  return w;
}

export function loadDrawing(kcad: string): CadDocument {
  const read = readSnapshot(kcad);
  if (!read.ok) throw new Error(read.error);
  const doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
  doc.replaceWith(read.content);
  return doc;
}

/** What the dialog shows, as the file writes it (NaN, a number field's unreadable text, is null). */
const plain = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/**
 * What changed from one view to the next, as a session's step writes it:
 * a changed part whole; `values` and `fields` by name, a field no longer
 * shown as null.
 */
export function viewDelta(prev: DialogView, next: DialogView): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(next) as (keyof DialogView)[]) {
    if (key === 'values' || key === 'fields') {
      const a = prev[key] as Record<string, unknown>;
      const b = next[key] as Record<string, unknown>;
      const d: Record<string, unknown> = {};
      for (const k of Object.keys(b)) if (!same(a[k], b[k])) d[k] = b[k];
      for (const k of Object.keys(a)) if (!(k in b)) d[k] = null;
      if (Object.keys(d).length) out[key] = d;
    } else if (!same(prev[key], next[key])) out[key] = next[key];
  }
  return out;
}

/** The view a step expects: the one before with the step's changes. */
export function applyDelta(prev: DialogView, delta: Record<string, unknown>): DialogView {
  const next: Record<string, unknown> = { ...prev };
  for (const [key, v] of Object.entries(delta)) {
    if (key === 'values') next.values = { ...prev.values, ...(v as Values) };
    else if (key === 'fields') {
      const fields: Record<string, unknown> = { ...prev.fields };
      for (const [name, f] of Object.entries(v as Values)) {
        if (f === null) delete fields[name];
        else fields[name] = f;
      }
      next.fields = fields;
    } else next[key] = v;
  }
  return next as unknown as DialogView;
}

/** Plays a session on the drawing: the view when it opens, then after each step. */
export async function playSession(spec: SessionSpec, kcad: string): Promise<DialogView[]> {
  const doc = loadDrawing(kcad);
  let selection = [...(spec.selection ?? [])];
  const bounds = spec.view ? { minX: spec.view[0], minY: spec.view[1], maxX: spec.view[2], maxY: spec.view[3] } : null;
  const executors: Executor[] = spec.available.map((t) => {
    if (t === 'client') return clientExecutor;
    if (t === 'worker') return workerExecutor(fakeWorker, new Set(TOOLS.keys()));
    throw new Error(`${spec.id}: bu yerde çalıştırıcı yok: ${t}`);
  });
  const runner = new ProcessingRunner({ doc, selectedIds: () => selection, visibleBounds: () => bounds, select: (ids) => (selection = [...ids]) }, executors);
  const format = new Formatter(doc.settings);
  const formatPoint = (p: Vec2) => format.point(p);

  const model = 'model' in spec.open ? MODELS.get(spec.open.model) : undefined;
  const tool: ProcessingTool | undefined = 'tool' in spec.open ? TOOLS.get(spec.open.tool) : model && modelAsTool(model, lookup);
  if (!tool) throw new Error(`${spec.id}: araç ya da model yok`);
  const modelTools = model ? model.steps.flatMap((s) => lookup(s.tool) ?? []) : undefined;
  const defaults = () => runner.defaults();
  const envOf = (s: DialogState) => hostEnv(runner, tool, s.values, doc.layers, formatPoint, modelTools);
  const paramOf = (name: string) => {
    const p = tool.parameters.find((x) => x.name === name);
    if (!p) throw new Error(`${spec.id}: parametre yok: ${name}`);
    return p;
  };

  let state = startState(tool, spec.given, spec.last, defaults(), spec.choice ?? 'auto');
  const views: DialogView[] = [plain(dialogView(tool, state, envOf(state)))];
  for (const step of spec.steps) {
    const a = step.do;
    if ('choose' in a) state = edited(state, a.choose.name, a.choose.value);
    else if ('type' in a) {
      const p = paramOf(a.type.name);
      const value = p.type === 'number' ? numberOfText(a.type.text) : p.type === 'layer' ? { newName: a.type.text } : a.type.text;
      state = edited(state, p.name, value, true);
    } else if ('scope' in a) state = edited(state, a.scope.name, withScope(state.values[a.scope.name] as FeaturesValue, a.scope.scope, doc.layers.active.value));
    else if ('scopeLayer' in a) state = edited(state, a.scopeLayer.name, withLayer(state.values[a.scopeLayer.name] as FeaturesValue, a.scopeLayer.layerId));
    else if ('kind' in a) {
      const present = envOf(state).inputs[a.kind.name]?.byKind ?? [];
      state = edited(state, a.kind.name, toggleKind(state.values[a.kind.name] as FeaturesValue, present, a.kind.kind));
    } else if ('toggleAdvanced' in a) state = toggledAdvanced(state);
    else if ('target' in a) state = chosenTarget(state, a.target);
    else if ('reset' in a) state = resetState(state, tool, defaults());
    else if ('undo' in a) {
      doc.undo();
      state = undone(state);
    } else if ('pick' in a) state = picked(state, a.pick.name, a.pick.point);
    else if ('run' in a) {
      const tried = attempted(state, tool, runner.validate(tool, state.values));
      state = tried.state;
      if (tried.run) {
        const target = effectiveChoice(state.choice, envOf(state).targets.available) ?? 'auto';
        const values = state.values;
        state = started(state, model ? undefined : runner.executorFor(tool, target, runner.inputSize(tool, values))?.target);
        const out: RunOutcome = model ? await runModel(model, values, runner, lookup, { target }) : await runner.run(tool, values, { target });
        state = finished(state, out);
      }
    }
    views.push(plain(dialogView(tool, state, envOf(state))));
  }
  return views;
}
