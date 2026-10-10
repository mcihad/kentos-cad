import type { Entity } from '../model/entities';
import type { LayerField } from '../model/layerFields';
import { compileExpression, type CompiledExpression, type ExprLayers } from '../model/expression/expression';
import { exprLayers, worldObjects } from '../model/expression/layers';
import type { ExpressionVariable } from '../model/projectVariables';
import { ObjectStore, type RunGeometry } from './geometry';
import type { DefaultsContext, DocumentSnapshot, ExecutionTarget, Feedback, FeatureSet, ProcessingTool, ProjectCrs, ProjectInfo, RunContext, RunResult } from './types';

/**
 * A run as data. The runner resolves what depends on the host (selection,
 * visible area, layers) into a RunJob that can be copied to a Web Worker
 * or sent to a server; `materialize` turns it back into what `run`
 * receives, against whatever document the executor has (the live one, or
 * a snapshot of its objects).
 */

/** A features input resolved to object ids. */
export interface FeatureRef {
  readonly ids: readonly number[];
  readonly description: string;
}

export interface RunJob {
  readonly toolId: string;
  /** Values keyed by parameter: features as FeatureRef, layers as TargetLayer, the rest as entered. */
  readonly values: Readonly<Record<string, unknown>>;
  readonly units: DefaultsContext;
  readonly selection: readonly number[];
  /** Layer id → name, for expressions and summaries. */
  readonly layers: readonly (readonly [string, string])[];
  /** The fields of the layers that have them (docs/adr/0199 §1): what a value written to an attribute becomes. */
  readonly fields: readonly (readonly [string, readonly LayerField[]])[];
  /** The project's coordinate system (docs/adr/0201 §8); null (or absent) without one. */
  readonly crs?: ProjectCrs | null;
  /** The project's SRID (0: none or a definition of its own) and type (docs/adr/0232 §12: the table's axes); absent: 0, none. */
  readonly project?: ProjectInfo;
  /** The `@` values its expressions read (docs/adr/0214 §2.3), taken when the run started; absent: none. */
  readonly variables?: readonly ExpressionVariable[];
  /** The objects the layers' filters leave out (docs/adr/0211 §1): the calls to other layers do not read them; absent: none. */
  readonly leftOut?: readonly number[];
}

/** The layers a job's calls to other objects read (docs/adr/0214 §3): their objects, but those the filters leave out. */
function jobLayers(job: RunJob, doc: DocumentSnapshot): ExprLayers {
  const out = job.leftOut?.length ? new Set(job.leftOut) : null;
  return exprLayers(job.layers, (id) => (out ? doc.byLayer(id).filter((e) => !out.has(e.id)) : doc.byLayer(id)));
}

/** Where a tool runs. `execute` gets the job and the document the page has; a remote one copies it. */
export interface Executor {
  readonly target: ExecutionTarget;
  available(): boolean;
  /** Whether this executor has the tool (a worker knows only the tools it was built with). */
  supports?(tool: ProcessingTool): boolean;
  execute(tool: ProcessingTool, job: RunJob, doc: DocumentSnapshot, feedback: Feedback): Promise<RunResult>;
}

const isFeatureRef = (v: unknown): v is FeatureRef => !!v && typeof v === 'object' && Array.isArray((v as FeatureRef).ids) && typeof (v as FeatureRef).description === 'string';

/**
 * The values `run` receives: features as objects of this document, expressions compiled with the run's `@` values
 * and with the calls that look at other layers (docs/adr/0214).
 */
export function materialize(tool: ProcessingTool, values: RunJob['values'], doc: DocumentSnapshot, variables: readonly ExpressionVariable[] = []): Record<string, unknown> {
  const out: Record<string, unknown> = { ...values };
  for (const p of tool.parameters) {
    const v = values[p.name];
    if (p.type === 'features' && isFeatureRef(v)) {
      const entities = v.ids.flatMap((id) => doc.get(id) ?? []);
      out[p.name] = { entities, description: v.description } satisfies FeatureSet;
    } else if (p.type === 'expression' && typeof v === 'string') {
      const src = v.trim();
      const r = src ? compileExpression(src, { variables, world: true }) : null;
      out[p.name] = r?.ok ? r.expr : null;
    } else if (p.type === 'field' && typeof v === 'string') out[p.name] = v.trim();
  }
  return out;
}

export function jobContext(job: RunJob, doc: DocumentSnapshot, geometry: RunGeometry, layers = jobLayers(job, doc)): RunContext {
  const names = new Map(job.layers);
  const order = new Map(job.layers.map(([id], k) => [id, k]));
  const fields = new Map(job.fields);
  return {
    doc,
    units: job.units,
    selection: job.selection,
    layerName: (id) => names.get(id) ?? id,
    geometry,
    field: (layerId, name) => fields.get(layerId)?.find((f) => f.name === name),
    crs: job.crs ?? null,
    project: job.project ?? { srid: 0, type: null },
    layerIndex: (id) => order.get(id) ?? Number.MAX_SAFE_INTEGER,
    layers,
  };
}

/** The objects of a tool's features inputs, each once. */
function inputObjects(tool: ProcessingTool, values: Record<string, unknown>): Entity[] {
  const byId = new Map<number, Entity>();
  for (const p of tool.parameters) {
    const set = p.type === 'features' ? (values[p.name] as FeatureSet | null | undefined) : null;
    for (const e of set?.entities ?? []) if (!byId.has(e.id)) byId.set(e.id, e);
  }
  return [...byId.values()];
}

/**
 * Runs a job against the document an executor has: the page's live one or
 * the worker's copies. Either way the objects the job reads go into a
 * geometry store of their own (docs/adr/0008, S4), so the page and the
 * worker run the same code; the store is freed when the run ends.
 */
export async function runJob(tool: ProcessingTool, job: RunJob, doc: DocumentSnapshot, feedback: Feedback): Promise<RunResult> {
  const values = materialize(tool, job.values, doc, job.variables);
  // The layers the expressions look at (docs/adr/0214 §3): their objects go into the run's store too.
  const layers = jobLayers(job, doc);
  const inputs = inputObjects(tool, values);
  const looked = tool.parameters.flatMap((p) => {
    const e = p.type === 'expression' ? (values[p.name] as CompiledExpression | null | undefined) : null;
    return e?.world ? worldObjects(e.world, inputs, layers) : [];
  });
  const byId = new Map<number, Entity>();
  for (const e of [...inputs, ...looked]) if (!byId.has(e.id)) byId.set(e.id, e);
  const geometry = new ObjectStore(byId.values());
  try {
    return await tool.run(values as never, jobContext(job, doc, geometry, layers), feedback);
  } finally {
    geometry.dispose();
  }
}

/** Read-only document made of copied objects (in a worker, or for a server request). */
export class EntitySnapshot implements DocumentSnapshot {
  private readonly byId = new Map<number, Entity>();
  private readonly layers = new Map<string, Entity[]>();

  constructor(entities: readonly Entity[]) {
    for (const e of entities) {
      this.byId.set(e.id, e);
      const list = this.layers.get(e.layerId);
      if (list) list.push(e);
      else this.layers.set(e.layerId, [e]);
    }
  }

  get(id: number): Entity | undefined {
    return this.byId.get(id);
  }

  all(): Iterable<Entity> {
    return this.byId.values();
  }

  byLayer(layerId: string): readonly Entity[] {
    return this.layers.get(layerId) ?? [];
  }
}

/** Runs the tool in the page, on the live document. */
export const clientExecutor: Executor = {
  target: 'client',
  available: () => true,
  execute: runJob,
};
