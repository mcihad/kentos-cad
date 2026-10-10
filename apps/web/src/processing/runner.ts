import { Signal } from '../core/signal';
import { foldTurkish } from '../core/text';
import type { Entity, NewEntity } from '../model/entities';
import { entityObjects, type BuilderObjects } from '../model/expression/builderObjects';
import { compileExpression, previewExpression, type ExprLayers } from '../model/expression/expression';
import { exprLayers, worldObjects } from '../model/expression/layers';
import { leftOut } from '../model/layerFilter';
import { documentVariables, type ExpressionVariable } from '../model/projectVariables';
import { crsSettings, datumChoices, ownSystem } from '../model/projectCrs';
import { resolveFeatures, resolveLayers, summarizeFeatures, summarizeLayers, type FeatureHost, type InputSummary } from './features';
import { withObjects } from './geometry';
import { clientExecutor, type Executor, type FeatureRef, type LayersRef, type RunJob } from './job';
import { fileTable, isVisible, validateValues, type ValidationIssue } from './parameters';
import type {
  DefaultsContext,
  ExecutionTarget,
  Feedback,
  FeaturesValue,
  FileValue,
  LayerParam,
  LayerValue,
  NetworkValue,
  ProcessingTool,
  ProjectCrs,
  ProjectInfo,
  RunResult,
  TargetLayer,
} from './types';
import { checkWrites } from './writeCheck';

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

/**
 * The layer a new-layer name writes to when a layer of that name exists:
 * names compare trimmed and Turkish-folded ("kose noktalari" is "Köşe
 * noktaları"). The dialog says "(mevcut)" by the same rule.
 */
export function sameNamedLayer<L extends { readonly name: string }>(leaves: readonly L[], name: string): L | undefined {
  const key = foldTurkish(name);
  return key ? leaves.find((l) => foldTurkish(l.name) === key) : undefined;
}

/** A new layer's id from its name (`islem-merkez`), “-2”, then “-2-3” … appended while `taken` says it is taken. */
export function newLayerId(name: string, taken: (id: string) => boolean): string {
  let id = `islem-${foldTurkish(name).toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  for (let k = 2; taken(id); k++) id = `${id}-${k}`;
  return id;
}

/** A name for a node among these: the name, or with “ (2)”, “ (3)” … when a node there has it (folded). */
export function freeName(name: string, siblings: readonly { readonly name: string }[]): string {
  let out = name;
  for (let k = 2; sameNamedLayer(siblings, out); k++) out = `${name} (${k})`;
  return out;
}

/** A chosen file's table as a field parameter reads it: its rows, and its named columns with how many rows fill each. */
function fileSummary(header: readonly string[], rows: readonly (readonly string[])[]): InputSummary {
  const fields: InputSummary['fields'] = [];
  header.forEach((name, i) => {
    if (name && !fields.some((f) => f.name === name)) fields.push({ name, count: rows.filter((r) => (r[i] ?? '').trim() !== '').length });
  });
  return { count: rows.length, description: '', byKind: [], fields, rows: true };
}

/** Where a scope looked, as the refusal of an input whose objects are all on locked layers says it. */
const LOCKED_WHERE: Record<Exclude<FeaturesValue['scope'], 'ids'>, string> = {
  selection: 'seçili nesnelerin',
  visible: 'görünen alandaki nesnelerin',
  all: 'görünen katmanlardaki nesnelerin',
  layer: 'bu katmandaki nesnelerin',
};

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
      drawingFont: s.drawingFont.value,
      activeLayer: this.host.doc.layers.active.value,
      measureHeightMm: s.annotationMm('measure'),
      // The project's networks, when it has any (docs/adr/0209): a network parameter's first.
      ...(s.networks.value.length && { networks: s.networks.value }),
    };
  }

  /** The project's coordinate system for a run (docs/adr/0201 §8): its own system and datum choices; null without one. */
  /** The project's SRID and type (docs/adr/0232). */
  projectInfo(): ProjectInfo {
    const settings = this.host.doc.settings;
    const w = settings.workspace.value;
    return { srid: settings.crs.value.srid, type: w === 'cad' || w === 'gis' ? w : null };
  }

  projectCrs(): ProjectCrs | null {
    const settings = crsSettings(this.host.doc.settings);
    const own = ownSystem(settings);
    return own?.system ? { srid: settings.srid, code: own.code, system: own.system, choices: datumChoices(settings) } : null;
  }

  validate(tool: ProcessingTool, values: Record<string, unknown>): ValidationIssue[] {
    const layers = this.host.doc.layers;
    const networks = this.host.doc.settings.networks.value;
    return validateValues(tool, values, {
      layerExists: (id) => !!layers.get(id),
      layerLocked: (id) => layers.isLocked(id),
      layerIsGroup: (id) => layers.get(id)?.type === 'group',
      network: (id) => networks.find((n) => n.id === id),
    });
  }

  /**
   * What each features parameter currently resolves to ("12 kapalı alan; seçili nesneler"), and each chosen file's
   * table (its columns as the fields a field parameter offers, docs/adr/0200 §6).
   */
  describeInputs(tool: ProcessingTool, values: Record<string, unknown>): Record<string, InputSummary> {
    const out: Record<string, InputSummary> = {};
    for (const p of tool.parameters) {
      if (p.type === 'features' && values[p.name]) out[p.name] = summarizeFeatures(values[p.name] as FeaturesValue, p, this.host);
      if (p.type === 'layers' && Array.isArray(values[p.name])) out[p.name] = summarizeLayers(values[p.name] as string[], this.host);
      if (p.type === 'file') {
        const t = fileTable(values[p.name] as FileValue | null);
        if (t) out[p.name] = fileSummary(t.header, t.rows);
      }
    }
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
    const r = src ? compileExpression(src, { variables: this.variables(), world: true }) : null;
    if (!r?.ok) return null;
    const input = tool.parameters.find((p) => p.name === (def.of ?? ''));
    if (input?.type !== 'features' || !values[input.name]) return null;
    const set = resolveFeatures(values[input.name] as FeaturesValue, input, this.host);
    // A raster's expression runs cell by cell in the raster core (docs/adr/0233 §3): no object to preview it on.
    if (onlyRasters(set.entities)) return null;
    const layers = this.host.doc.layers;
    const layerName = (id: string) => layers.get(id)?.name ?? id;
    // Evaluated in the drawing's store the host keeps, else in a store of the previewed objects: the geometry values are read there.
    const geometry = this.host.geometry;
    // The layers the calls to other objects look at (docs/adr/0214 §3); a store of their own has their objects too.
    const looked = this.layers();
    return geometry
      ? previewExpression(r.expr, set.entities, def.returns, layerName, undefined, geometry, looked)
      : withObjects([...set.entities, ...worldObjects(r.expr.world, set.entities, looked)], (s) =>
          previewExpression(r.expr, set.entities, def.returns, layerName, undefined, s, looked),
        );
  }

  /** The `@` values the run's expressions read (docs/adr/0214 §2.3), now. */
  variables(): readonly ExpressionVariable[] {
    return this.host.variables?.() ?? documentVariables(this.host.doc, new Date(), '');
  }

  /**
   * The drawing's layers as the calls to other objects read them (docs/adr/0214 §3): a layer's filter leaves its
   * objects out here as it does of the inputs (docs/adr/0211 §1), asked once, when a layer is first read.
   */
  private layers(): ExprLayers {
    const doc = this.host.doc;
    let out: Set<number> | null = null;
    return exprLayers(
      doc.layers.leaves().map((l) => [l.id, l.name] as const),
      (id) => {
        out ??= leftOut(doc);
        const left = out;
        return left.size ? doc.byLayer(id).filter((e) => !left.has(e.id)) : doc.byLayer(id);
      },
    );
  }

  /**
   * The objects an expression parameter runs on, for the expression
   * builder's preview and a field's values (docs/adr/0100 §5); undefined
   * while its input resolves to nothing.
   */
  builderObjects(tool: ProcessingTool, values: Record<string, unknown>, name: string): BuilderObjects | undefined {
    const def = tool.parameters.find((p) => p.name === name);
    if (def?.type !== 'expression') return undefined;
    const input = tool.parameters.find((p) => p.name === (def.of ?? ''));
    if (input?.type !== 'features' || !values[input.name]) return undefined;
    const set = resolveFeatures(values[input.name] as FeaturesValue, input, this.host);
    if (onlyRasters(set.entities)) return undefined;
    const layers = this.host.doc.layers;
    // The drawing's store the host keeps gives every geometry value ($genişlik, $merkez_y …); the measures are the fallback.
    return entityObjects(set.entities, (id) => layers.get(id)?.name ?? id, {
      geometry: this.host.geometry,
      measures: (list) => this.measures(list),
      context: { variables: this.variables(), world: true },
      layers: this.layers(),
    });
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

  /** A layer's objects: what a Katmanlar list says beside it (docs/adr/0217 §7). */
  layerCount(id: string): number {
    return this.host.doc.byLayer(id).length;
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
    const newLayers = new Map<string, { name: string; def: LayerParam; above?: string; below?: string }>();
    /** What resolving the inputs left out, said once the run starts. */
    const notes: string[] = [];
    let size = 0;
    for (const p of tool.parameters) {
      const v = values[p.name];
      if (!isVisible(p, values) || v === null || v === undefined) continue;
      if (p.type === 'features') {
        const fv = v as FeaturesValue;
        const set = resolveFeatures(fv, p, this.host);
        let entities = set.entities;
        // A tool that changes its input does not get objects it may not change (layers locked).
        const locked = p.writes ? entities.filter((e) => this.host.doc.layers.isLocked(e.layerId)).length : 0;
        if (locked) {
          entities = entities.filter((e) => !this.host.doc.layers.isLocked(e.layerId));
          if (!entities.length && !p.optional && fv.scope !== 'ids')
            return { status: 'invalid', issues: [{ param: p.name, message: `“${p.label}”: ${LOCKED_WHERE[fv.scope]} hepsi kilitli katmanda. Kilidi Katmanlar panelinden açın.` }] };
          notes.push(`“${p.label}”: ${locked} nesne kilitli katmanda olduğu için işleme alınmadı.`);
        }
        // Running on nothing is a mistake worth stopping (usually: nothing selected); an empty output passed along a
        // model is not, nor an input that may be empty (a layer a run may only add to, docs/adr/0216 §3).
        if (!entities.length && !p.optional && !p.allowEmpty && fv.scope !== 'ids') return { status: 'invalid', issues: [{ param: p.name, message: emptyInputMessage(p.label, fv) }] };
        size += entities.length;
        jobValues[p.name] = { ids: entities.map((e) => e.id), description: set.description, ...(fv.scope === 'layer' && { layerId: fv.layerId }) } satisfies FeatureRef;
      }
      // A list of layers (docs/adr/0217 §7): read in the tree's order; a locked layer's objects are not taken.
      if (p.type === 'layers') {
        const sets = resolveLayers(v as string[], this.host);
        const taken = sets.filter((l) => !this.host.doc.layers.isLocked(l.id));
        const locked = sets.reduce((n, l) => n + (taken.includes(l) ? 0 : l.entities.length), 0);
        if (locked) notes.push(`“${p.label}”: ${locked} nesne kilitli katmanda olduğu için işleme alınmadı.`);
        const count = taken.reduce((n, l) => n + l.entities.length, 0);
        if (!count && !p.optional) return { status: 'invalid', issues: [{ param: p.name, message: `“${p.label}”: seçilen katmanlarda uygun nesne yok.` }] };
        size += count;
        jobValues[p.name] = { layers: taken.map((l) => ({ id: l.id, name: l.name, ids: l.entities.map((e) => e.id) })) } satisfies LayersRef;
      }
      if (p.type === 'layer') {
        const t = this.resolveLayer(v as LayerValue);
        if (t.isNew) newLayers.set(t.id, { name: t.name, def: p });
        jobValues[p.name] = t;
      }
      // A network goes with its definition as it is now (docs/adr/0209 §10): the run builds it from the objects.
      if (p.type === 'network') {
        const n = v as NetworkValue;
        jobValues[p.name] = { network: n.network, cost: n.cost, def: this.host.doc.settings.networks.value.find((d) => d.id === n.network) } satisfies NetworkValue;
      }
    }
    // A new layer named to go above or below a features input: the layer of that input's first object. An input not shown
    // is not resolved, and names none (Uzaklık yüzeyi's objects or raster, docs/adr/0236 §2).
    const layerOf = (param: string | undefined) => {
      const p = param ? tool.parameters.find((d) => d.name === param) : undefined;
      if (!p || !isVisible(p, values)) return undefined;
      const first = (jobValues[param!] as FeatureRef | undefined)?.ids?.[0];
      return first === undefined ? undefined : this.host.doc.get(first)?.layerId;
    };
    for (const plan of newLayers.values()) {
      plan.above = layerOf(plan.def.above);
      plan.below = layerOf(plan.def.below);
    }
    const executor = this.executorFor(tool, choice, size);
    if (!executor) {
      const where = choice === 'auto' ? tool.targets.join(', ') : choice;
      const message = `“${tool.label}” bu ortamda çalıştırılamıyor (${where} gerekli).`;
      return { status: 'error', message, record: record('error', message) };
    }
    target = executor.target;
    const layers = this.host.doc.layers;
    // What the layers' filters leave out (docs/adr/0211 §1): the calls to other layers do not read them (docs/adr/0214 §3).
    const out = leftOut(this.host.doc);
    const job: RunJob = {
      toolId: tool.id,
      values: jobValues,
      units: this.defaults(),
      selection: [...this.host.selectedIds()],
      layers: layers.leaves().map((l) => [l.id, l.name] as const),
      fields: layers.leaves().flatMap((l) => (l.fields?.length ? [[l.id, l.fields] as const] : [])),
      crs: this.projectCrs(),
      project: this.projectInfo(),
      variables: this.variables(),
      ...(out.size ? { leftOut: [...out] } : {}),
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
    for (const n of notes) log?.('warn', n);
    this.running.set({ toolId: tool.id, fraction: 0, label: '' });
    try {
      let result = await executor.execute(tool, job, this.host.doc, feedback);
      if (this.canceled) {
        const message = 'İşlem iptal edildi; çizim değişmedi.';
        return { status: 'canceled', message, record: record('canceled', message) };
      }
      // A tool that refuses, or a value a layer's field does not take, ends the run with nothing changed.
      const checked = result.refused === undefined && result.changes ? checkWrites(this.host.doc, result.changes) : null;
      const refused = result.refused ?? (checked && 'refused' in checked ? checked.refused : undefined);
      if (refused !== undefined) return { status: 'error', message: refused, record: record('error', refused) };
      if (checked && 'changes' in checked) result = { ...result, changes: checked.changes };
      const added = this.apply(tool, result, newLayers, log);
      const doc = this.host.doc;
      const selected = result.select ? [...new Set(result.select)].filter((id) => doc.get(id)) : null;
      if (selected) this.host.select?.(selected);
      const touched = [...new Set([...(result.changes?.update ?? []).map((u) => u.id), ...(selected ?? [])])].filter((id) => doc.get(id));
      const summary = result.summary ?? (selected ? `${selected.length} nesne seçildi.` : `${added.length} nesne eklendi.`);
      const ch = result.changes;
      const edited = added.length > 0 || !!ch?.update?.length || !!ch?.remove?.length || !!result.layers?.length;
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
    const same = sameNamedLayer(layers.leaves(), name);
    if (same) return { id: same.id, name: same.name, isNew: false };
    return { id: newLayerId(name, (id) => !!layers.get(id)), name, isNew: true };
  }

  /** Applies the ChangeSet in one undo step; locked layers are left alone and counted. */
  private apply(tool: ProcessingTool, result: RunResult, newLayers: Map<string, { name: string; def: LayerParam; above?: string; below?: string }>, log?: (level: 'info' | 'warn', message: string) => void): number[] {
    const { doc } = this.host;
    if (!result.changes && !result.layers?.length) return [];
    const ch = result.changes ?? {};
    const locked = (layerId: string) => doc.layers.isLocked(layerId);
    let skipped = 0;
    const added: number[] = [];
    // A new layer of the run's layer parameters takes this layer's look and fields (docs/adr/0217 §7).
    const likeOf = (id: string | undefined) => {
      const like = id ? doc.layers.get(id) : undefined;
      return like?.type === 'layer' ? { style: like.style, ...(like.fields?.length && { fields: like.fields }) } : null;
    };
    // Removals, then updates, then additions, each as one change (the panels and the store hear it once).
    doc.transact(tool.label, () => {
      // Layers and groups the run makes itself (docs/adr/0217 §7), in its order: a key becomes an id from the name, a
      // name taken where it goes gets “ (2)”; a group's layers go into it, first on top.
      const keys = new Map<string, string>();
      for (const t of result.layers ?? []) {
        const parent = t.parent !== undefined ? (keys.get(t.parent) ?? null) : null;
        const anchor = parent === null && t.above && doc.layers.get(t.above) ? t.above : null;
        const group = parent ?? (anchor ? (doc.layers.parentOf(anchor)?.id ?? null) : null);
        const siblings = group ? (doc.layers.get(group)?.children ?? []) : doc.layers.tree;
        const name = freeName(t.name, siblings);
        const id = newLayerId(name, (x) => !!doc.layers.get(x));
        const index = anchor ? siblings.findIndex((n) => n.id === anchor) : -1;
        const look = t.group ? null : likeOf(t.like);
        doc.addLayer({ id, name, type: t.group ? 'group' : 'layer', ...look }, group, index >= 0 ? { index } : {});
        keys.set(t.key, id);
      }
      const real = (layerId: string) => keys.get(layerId) ?? layerId;
      // Layers the run writes to but that do not exist yet: in the tool's step, so undo takes them too.
      // New layers below one layer keep the run's order: each below the one before (Kriging's prediction right under
      // its points, its error under that).
      const under = new Map<string, number>();
      for (const e of ch.add ?? []) {
        const pending = newLayers.get(e.layerId);
        if (!pending || doc.layers.get(e.layerId)) continue;
        // Right above or below the layer its plan names (its group, its place), else last; the tool may name it.
        const above = pending.above && result.above ? result.above : pending.above;
        const over = above && doc.layers.get(above) ? above : null;
        const below = !over && pending.below && doc.layers.get(pending.below) ? pending.below : null;
        const anchor = over ?? below;
        const group = anchor ? doc.layers.parentOf(anchor) : null;
        let index = anchor ? (group ? group.children : doc.layers.tree).findIndex((n) => n.id === anchor) : -1;
        if (below && index >= 0) {
          const n = (under.get(below) ?? 0) + 1;
          under.set(below, n);
          index += n;
        }
        doc.addLayer({ id: e.layerId, name: pending.name, ...(likeOf(result.like) ?? { style: pending.def.newLayerStyle }) }, group?.id ?? null, index >= 0 ? { index } : {});
      }
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
        else patches.push({ ...(u.patch as Partial<Entity>), id: u.id, ...(u.patch.layerId !== undefined && { layerId: real(u.patch.layerId) }) });
      }
      doc.updateMany(patches);
      const fresh: NewEntity[] = [];
      for (const given of ch.add ?? []) {
        const n = keys.has(given.layerId) ? { ...given, layerId: real(given.layerId) } : given;
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

/** Rasters only: an expression on them names their bands (docs/adr/0233 §3), not attributes. */
function onlyRasters(list: readonly Entity[]): boolean {
  return list.length > 0 && list.every((e) => e.kind === 'raster');
}
