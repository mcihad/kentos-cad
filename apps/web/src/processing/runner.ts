import { Signal } from '../core/signal';
import { foldTurkish } from '../core/text';
import type { Entity, NewEntity } from '../model/entities';
import { compileExpression, previewExpression } from '../model/expression/expression';
import { resolveFeatures, summarizeFeatures, type FeatureHost, type InputSummary } from './features';
import { withObjects } from './geometry';
import { clientExecutor, type Executor, type FeatureRef, type RunJob } from './job';
import { isVisible, validateValues, type ValidationIssue } from './parameters';
import type { DefaultsContext, ExecutionTarget, Feedback, FeaturesValue, LayerParam, LayerValue, ProcessingTool, RunResult, TargetLayer } from './types';

/**
 * Runs processing tools: validate → resolve what depends on the host into
 * a RunJob → execute where the user or the size of the job says (in the
 * page, or a Web Worker) → apply the ChangeSet as one undo step → record
 * history. Server and PostGIS executors plug in through the same
 * Executor interface (job.ts).
 */

export { clientExecutor, type Executor } from './job';

/** Where the user wants a tool to run; "auto" sends big jobs to the worker. */
export type TargetChoice = 'auto' | ExecutionTarget;

export interface RunOptions {
  /** Messages from the tool (feedback.info / warn) and about skipped changes. */
  log?: (level: 'info' | 'warn', message: string) => void;
  target?: TargetChoice;
  /** No history record (a model records itself once, not each step). */
  silent?: boolean;
}

/** Input size (objects) from which "auto" prefers the worker. */
export const WORKER_THRESHOLD = 2000;

export interface RunRecord {
  readonly seq: number;
  readonly toolId: string;
  readonly label: string;
  /** Values as entered (JSON-safe copy), for "run again". */
  readonly values: Record<string, unknown>;
  readonly started: number;
  readonly ms: number;
  readonly status: 'ok' | 'error' | 'canceled';
  readonly summary: string;
  /** Ids of objects the run created. */
  readonly added: readonly number[];
  /** Ids the run changed or selected (what "Sonuçları seç" offers when nothing was added). */
  readonly touched: readonly number[];
  /** Where it ran; absent when it did not start. */
  readonly target?: ExecutionTarget;
}

export type RunOutcome =
  /** `edited`: the drawing changed (there is something to undo). */
  | { status: 'ok'; result: RunResult; added: number[]; touched: number[]; edited: boolean; record: RunRecord }
  | { status: 'invalid'; issues: ValidationIssue[] }
  | { status: 'canceled' | 'error'; message: string; record: RunRecord };

export type { InputSummary } from './features';

export interface RunProgress {
  toolId: string;
  fraction: number;
  label: string;
}

const HISTORY_LIMIT = 100;

function emptyInputMessage(label: string, v: FeaturesValue): string {
  switch (v.scope) {
    case 'selection':
      return `“${label}”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin.`;
    case 'visible':
      return `“${label}”: görünen alanda uygun nesne yok. Görünümü kaydırın ya da kapsamı değiştirin.`;
    case 'layer':
      return `“${label}”: bu katmanda uygun nesne yok. Başka bir katman seçin.`;
    default:
      return `“${label}”: uygun nesne yok.`;
  }
}

export class ProcessingRunner {
  readonly running = new Signal<RunProgress | null>(null);
  readonly history = new Signal<readonly RunRecord[]>([]);
  private readonly host: FeatureHost;
  private readonly executors: Executor[];
  private canceled = false;
  private seq = 0;

  constructor(host: FeatureHost, executors: Executor[] = [clientExecutor]) {
    this.host = host;
    this.executors = executors;
  }

  defaults(): DefaultsContext {
    const s = this.host.doc.settings;
    return {
      lengthDecimals: s.lengthDecimals.value,
      areaDecimals: s.areaDecimals.value,
      angleUnit: s.angleUnit.value,
      plotScale: s.plotScale.value,
      activeLayer: this.host.doc.layers.active.value,
    };
  }

  validate(tool: ProcessingTool, values: Record<string, unknown>): ValidationIssue[] {
    const layers = this.host.doc.layers;
    return validateValues(tool, values, { layerExists: (id) => !!layers.get(id), layerLocked: (id) => layers.isLocked(id) });
  }

  /** What each features parameter currently resolves to ("12 kapalı alan; seçili nesneler"). */
  describeInputs(tool: ProcessingTool, values: Record<string, unknown>): Record<string, InputSummary> {
    const out: Record<string, InputSummary> = {};
    for (const p of tool.parameters) if (p.type === 'features' && values[p.name]) out[p.name] = summarizeFeatures(values[p.name] as FeaturesValue, p, this.host);
    return out;
  }

  /**
   * How an expression parameter works out on the objects it reads, for the
   * dialog: "12 / 68 nesne koşulu sağlıyor." or the first value. Null when
   * the expression is empty or has an error (the field shows that).
   */
  previewExpression(tool: ProcessingTool, values: Record<string, unknown>, name: string): string | null {
    const def = tool.parameters.find((p) => p.name === name);
    if (def?.type !== 'expression') return null;
    const src = String(values[name] ?? '').trim();
    const r = src ? compileExpression(src) : null;
    if (!r?.ok) return null;
    const input = tool.parameters.find((p) => p.name === (def.of ?? ''));
    if (input?.type !== 'features' || !values[input.name]) return null;
    const set = resolveFeatures(values[input.name] as FeaturesValue, input, this.host);
    const layers = this.host.doc.layers;
    return previewExpression(r.expr, set.entities, def.returns, (id) => layers.get(id)?.name ?? id, (list) => this.measures(list));
  }

  /** The expressions' geometry values of these objects: from the drawing's store the host keeps, else from a store of their own. */
  private measures(list: readonly Entity[]): Float64Array {
    const ids = list.map((e) => e.id);
    const geometry = this.host.geometry;
    return geometry ? geometry.measures(ids) : withObjects(list, (s) => s.measures(ids));
  }

  /** Executors here that can run the tool, in the tool's order of preference. */
  executorsFor(tool: ProcessingTool): Executor[] {
    return tool.targets.flatMap((t) => this.executors.filter((e) => e.target === t && e.available() && (e.supports?.(tool) ?? true)));
  }

  /**
   * The executor for this run: the one the user chose, or on "auto" the
   * worker for inputs of WORKER_THRESHOLD objects and more, else the
   * tool's first choice. Null when none can run it here.
   */
  executorFor(tool: ProcessingTool, choice: TargetChoice = 'auto', size = 0): Executor | null {
    const all = this.executorsFor(tool);
    if (choice !== 'auto') return all.find((e) => e.target === choice) ?? null;
    if (size >= WORKER_THRESHOLD) {
      const worker = all.find((e) => e.target === 'worker');
      if (worker) return worker;
    }
    return all[0] ?? null;
  }

  /** Objects the features inputs resolve to now (what "auto" weighs). */
  inputSize(tool: ProcessingTool, values: Record<string, unknown>): number {
    return Object.values(this.describeInputs(tool, values)).reduce((n, s) => n + s.count, 0);
  }

  cancel(): void {
    this.canceled = true;
  }

  /** Whether the user pressed Durdur during the current run (models stop between steps). */
  get canceling(): boolean {
    return this.canceled;
  }

  /** One undo step for several runs (a model); see CadDocument.beginGroup. */
  beginGroup(label: string): { end(): void; cancel(): void } {
    return this.host.doc.beginGroup(label);
  }

  /** Adds a history record (runs record themselves; a model adds one for all its steps). */
  addRecord(r: Omit<RunRecord, 'seq'>): RunRecord {
    const full: RunRecord = { ...r, seq: ++this.seq };
    this.history.set([full, ...this.history.value].slice(0, HISTORY_LIMIT));
    return full;
  }

  async run(tool: ProcessingTool, values: Record<string, unknown>, opts: RunOptions = {}): Promise<RunOutcome> {
    const { log, target: choice = 'auto', silent = false } = opts;
    const issues = this.validate(tool, values);
    if (issues.length) return { status: 'invalid', issues };
    const started = Date.now();
    const copy = JSON.parse(JSON.stringify(values)) as Record<string, unknown>;
    let target: ExecutionTarget | undefined;
    const record = (status: RunRecord['status'], summary: string, added: number[] = [], touched: number[] = []): RunRecord => {
      const r = { toolId: tool.id, label: tool.label, values: copy, started, ms: Date.now() - started, status, summary, added, touched, target };
      return silent ? { ...r, seq: 0 } : this.addRecord(r);
    };
    // Resolve what depends on the host: features to ids, layers to a target (new layers are made only on apply).
    const jobValues: Record<string, unknown> = { ...values };
    const newLayers = new Map<string, { name: string; def: LayerParam }>();
    let size = 0;
    for (const p of tool.parameters) {
      const v = values[p.name];
      if (!isVisible(p, values) || v === null || v === undefined) continue;
      if (p.type === 'features') {
        const set = resolveFeatures(v as FeaturesValue, p, this.host);
        // Running on nothing is a mistake worth stopping (usually: nothing selected);
        // an empty output passed along a model is not.
        if (!set.entities.length && !p.optional && (v as FeaturesValue).scope !== 'ids') return { status: 'invalid', issues: [{ param: p.name, message: emptyInputMessage(p.label, v as FeaturesValue) }] };
        size += set.entities.length;
        jobValues[p.name] = { ids: set.entities.map((e) => e.id), description: set.description } satisfies FeatureRef;
      }
      if (p.type === 'layer') {
        const t = this.resolveLayer(v as LayerValue);
        if (t.isNew) newLayers.set(t.id, { name: t.name, def: p });
        jobValues[p.name] = t;
      }
    }
    const executor = this.executorFor(tool, choice, size);
    if (!executor) {
      const where = choice === 'auto' ? tool.targets.join(', ') : choice;
      const message = `“${tool.label}” bu ortamda çalıştırılamıyor (${where} gerekli).`;
      return { status: 'error', message, record: record('error', message) };
    }
    target = executor.target;
    const layers = this.host.doc.layers;
    const job: RunJob = {
      toolId: tool.id,
      values: jobValues,
      units: this.defaults(),
      selection: [...this.host.selectedIds()],
      layers: layers.leaves().map((l) => [l.id, l.name] as const),
    };

    this.canceled = false;
    let lastYield = performance.now();
    const isCanceled = () => this.canceled;
    const feedback: Feedback = {
      progress: (fraction, label) => this.running.set({ toolId: tool.id, fraction: Math.max(0, Math.min(1, fraction)), label: label ?? '' }),
      info: (m) => log?.('info', m),
      warn: (m) => log?.('warn', m),
      get canceled() {
        return isCanceled();
      },
      yield: () => {
        if (performance.now() - lastYield < 16) return Promise.resolve();
        return new Promise((r) =>
          setTimeout(() => {
            lastYield = performance.now();
            r();
          }, 0),
        );
      },
    };
    this.running.set({ toolId: tool.id, fraction: 0, label: '' });
    try {
      const result = await executor.execute(tool, job, this.host.doc, feedback);
      if (this.canceled) {
        const message = 'İşlem iptal edildi; çizim değişmedi.';
        return { status: 'canceled', message, record: record('canceled', message) };
      }
      const added = this.apply(tool, result, newLayers, log);
      const doc = this.host.doc;
      const selected = result.select ? [...new Set(result.select)].filter((id) => doc.get(id)) : null;
      if (selected) this.host.select?.(selected);
      const touched = [...new Set([...(result.changes?.update ?? []).map((u) => u.id), ...(selected ?? [])])].filter((id) => doc.get(id));
      const summary = result.summary ?? (selected ? `${selected.length} nesne seçildi.` : `${added.length} nesne eklendi.`);
      const ch = result.changes;
      const edited = added.length > 0 || !!ch?.update?.length || !!ch?.remove?.length;
      return { status: 'ok', result, added, touched, edited, record: record('ok', summary, added, touched) };
    } catch (err) {
      const message = `“${tool.label}” çalışırken hata: ${(err as Error).message}`;
      return { status: 'error', message, record: record('error', message) };
    } finally {
      this.running.set(null);
    }
  }

  /**
   * A new-layer value reuses a layer that already has that name (running a
   * tool twice keeps writing to the same "Köşe noktaları"); otherwise it
   * gets an id now and is created on apply.
   */
  private resolveLayer(v: LayerValue): TargetLayer {
    const layers = this.host.doc.layers;
    if ('layerId' in v) return { id: v.layerId, name: layers.get(v.layerId)?.name ?? v.layerId, isNew: false };
    const name = v.newName.trim();
    const same = layers.leaves().find((l) => foldTurkish(l.name) === foldTurkish(name));
    if (same) return { id: same.id, name: same.name, isNew: false };
    let id = `islem-${foldTurkish(name).toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
    for (let k = 2; layers.get(id); k++) id = `${id}-${k}`;
    return { id, name, isNew: true };
  }

  /** Applies the ChangeSet in one undo step; locked layers are left alone and counted. */
  private apply(tool: ProcessingTool, result: RunResult, newLayers: Map<string, { name: string; def: LayerParam }>, log?: (level: 'info' | 'warn', message: string) => void): number[] {
    const { doc } = this.host;
    const ch = result.changes;
    if (!ch) return [];
    // Layers the run writes to but that do not exist yet.
    for (const e of ch.add ?? []) {
      const pending = newLayers.get(e.layerId);
      if (!pending || doc.layers.get(e.layerId)) continue;
      doc.layers.add({ id: e.layerId, name: pending.name, style: pending.def.newLayerStyle }, null);
    }
    const locked = (layerId: string) => doc.layers.isLocked(layerId);
    let skipped = 0;
    const added: number[] = [];
    // Removals, then updates, then additions, each as one change (the panels and the store hear it once).
    doc.transact(tool.label, () => {
      const gone: number[] = [];
      for (const id of ch.remove ?? []) {
        const e = doc.get(id);
        if (!e) continue;
        if (locked(e.layerId)) skipped++;
        else gone.push(id);
      }
      doc.remove(gone);
      const patches: (Partial<Entity> & { id: number })[] = [];
      for (const u of ch.update ?? []) {
        const e = doc.get(u.id);
        if (!e) continue;
        if (locked(e.layerId)) skipped++;
        else patches.push({ ...(u.patch as Partial<Entity>), id: u.id });
      }
      doc.updateMany(patches);
      const fresh: NewEntity[] = [];
      for (const n of ch.add ?? []) {
        if (!doc.layers.get(n.layerId) || locked(n.layerId)) {
          skipped++;
          continue;
        }
        fresh.push(n as NewEntity);
      }
      for (const e of doc.addMany(fresh)) added.push(e.id);
    });
    if (skipped) log?.('warn', `${skipped} değişiklik kilitli ya da olmayan katmanda olduğu için atlandı.`);
    return added;
  }
}
