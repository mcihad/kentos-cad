import { describe, expect, it } from 'vitest';
import { canFeed, checkModel, orderSteps, type ModelIssue, type ProcessingModel, type ValueSource } from '../../../processing/model';
import {
  addInput,
  addOutput,
  addStep,
  autoLayout,
  copyModel,
  edgesOf,
  INPUT_TYPES,
  inputFromParam,
  newModel,
  removeInput,
  removeStep,
  setCaption,
  setSource,
  slug,
  sourcesFor,
  type ModelInputType,
} from '../../../processing/modelEdit';
import type { DefaultsContext, OutputDef, ParamDef, ProcessingTool, Shown } from '../../../processing/types';
import {
  boxesBounds,
  CANVAS,
  COALESCE_MS,
  connectChoices,
  curve,
  curveMid,
  DESIGNER_HISTORY,
  DESIGNER_TEXTS,
  designerStatus,
  edgeLabel,
  fitView,
  inputPort,
  joins,
  savedLabel,
  snap,
  sourceText,
  spotNear,
  stepEntry,
  stepMeta,
  stepPort,
  zoomAt,
  type NodeRef,
  type Pt,
  type View,
} from './designerPlan';

/**
 * The model designer (fixtures/processing/v1/designer.json, format in
 * fixtures/processing/README.md; docs/specs/model-designer.md): the model's
 * edits replayed step by step with the checks after each, what the designer
 * reads off a model (sources, edges, order, box texts, the wire menu), its
 * words, and the diagram's geometry. The file's answers are worked out apart
 * from this code (scripts/fixtures/designer_cases.py), on the file's own
 * tools; the desktop's designer checks itself against the same file.
 */

const files = import.meta.glob<string>('../../../../../../fixtures/processing/v1/designer.json', { query: '?raw', import: 'default', eager: true });
type FixtureParam = Record<string, unknown> & { name: string; visibleWhen?: { param: string; equals: unknown } };
type Op =
  | { op: 'addInput'; type: ModelInputType; label: string; at?: Pt }
  | { op: 'addStep'; tool: string; at?: Pt; from?: { kind: 'input'; name: string } | { kind: 'step'; id: string } }
  | { op: 'setSource'; step: string; param: string; src: ValueSource | null }
  | { op: 'removeInput'; name: string }
  | { op: 'removeStep'; id: string }
  | { op: 'inputFromParam'; step: string; param: string }
  | { op: 'addOutput'; step: string; output: string }
  | { op: 'caption'; step: string; caption: string }
  | { op: 'autoLayout' };
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  texts: unknown;
  inputTypes: unknown;
  canvas: unknown;
  history: { depth: number; coalesceMs: number };
  tools: { id: string; label: string; icon: string; parameters: FixtureParam[]; outputs: OutputDef[] }[];
  defaults: DefaultsContext;
  canFeed: { from: string; feeds: string[] }[];
  slugs: { label: string; slug: string }[];
  newModel: unknown;
  copyLabel: { label: string; copy: string };
  sequences: { title: string; steps: { op: Op; result: unknown; model: unknown; issues: ModelIssue[] }[] }[];
  models: {
    title: string;
    model: ProcessingModel;
    status: unknown;
    order: unknown;
    edges: unknown;
    edgeLabels: string[];
    stepMeta: { step: string; meta: unknown }[];
    sources: { step: string; param: string; options: unknown; text: string }[];
    wires: { from: NodeRef; to: string; choices: unknown }[];
    bounds: unknown;
  }[];
  spots: { selected: NodeRef | null; empty?: boolean; spot: Pt }[];
  titles: { label: string; dirty: boolean; title: string }[];
  savedLabels: { label: string; saved: string }[];
  joins: { key: string | null; at: number; joins: boolean }[];
  geometry: {
    curves: { a: Pt; b: Pt; curve: unknown; mid: unknown }[];
    ports: { input: { at: Pt; port: Pt }; step: { at: Pt; port: Pt; entry: Pt } };
    fits: { bounds: { x: number; y: number; w: number; h: number } | null; width: number; height: number; view: View }[];
    zooms: { view: View; at: Pt; factor: number; result: View }[];
    snaps: { value: number; snapped: number }[];
  };
};

/** The file's tools as the registry would hold them: `visibleWhen` from its data form. */
const TOOLS = new Map(
  F.tools.map((t) => {
    const parameters = t.parameters.map((p) => {
      const when = p.visibleWhen;
      return (when ? { ...p, visibleWhen: (v: Shown) => v[when.param] === when.equals } : p) as unknown as ParamDef;
    });
    const tool = { id: t.id, label: t.label, category: 'points', description: '', icon: t.icon, parameters, outputs: t.outputs, targets: ['client'], run: () => ({}) };
    return [t.id, tool as unknown as ProcessingTool] as const;
  }),
);
const lookup = (id: string) => TOOLS.get(id);
const plain = (v: unknown): unknown => (v === undefined ? null : JSON.parse(JSON.stringify(v)));

function apply(model: ProcessingModel, op: Op): unknown {
  switch (op.op) {
    case 'addInput':
      return addInput(model, op.type, op.label, op.at);
    case 'addStep':
      return addStep(model, op.tool, lookup, op.at, op.from);
    case 'setSource':
      return setSource(model, op.step, op.param, op.src);
    case 'removeInput':
      return removeInput(model, op.name);
    case 'removeStep':
      return removeStep(model, op.id);
    case 'inputFromParam': {
      const step = model.steps.find((s) => s.id === op.step);
      const p = step && lookup(step.tool)?.parameters.find((x) => x.name === op.param);
      return p ? inputFromParam(model, op.step, p, F.defaults) : null;
    }
    case 'addOutput':
      return addOutput(model, op.step, op.output, lookup);
    case 'caption':
      return setCaption(model, op.step, op.caption);
    case 'autoLayout':
      return autoLayout(model);
  }
}

/** Texts as the file writes them: a text made from values as `{ sample: [...], text }`. */
function fileTexts(o: object, samples: Record<string, unknown[]>, path = ''): unknown {
  return Object.fromEntries(
    Object.entries(o).map(([k, v]) => {
      const at = path ? `${path}.${k}` : k;
      if (typeof v === 'function') {
        const sample = samples[at];
        if (!sample) throw new Error(`Metin örneği yok: ${at}`);
        return [k, { sample, text: (v as (...a: unknown[]) => string)(...sample) }];
      }
      return [k, v && typeof v === 'object' && !Array.isArray(v) ? fileTexts(v as object, samples, at) : v];
    }),
  );
}
function samplesOf(o: unknown, path = '', out: Record<string, unknown[]> = {}): Record<string, unknown[]> {
  if (o && typeof o === 'object' && !Array.isArray(o)) {
    const r = o as Record<string, unknown>;
    if (Array.isArray(r.sample) && typeof r.text === 'string') out[path] = r.sample;
    else for (const [k, v] of Object.entries(r)) samplesOf(v, path ? `${path}.${k}` : k, out);
  }
  return out;
}

describe('the model designer (fixtures/processing/v1/designer.json)', () => {
  it('is a v1 designer file with the designer’s words, input kinds, diagram numbers and undo', () => {
    expect([F.format, F.version]).toEqual(['kentos.modelDesigner', 1]);
    expect(fileTexts(DESIGNER_TEXTS, samplesOf(F.texts))).toEqual(F.texts);
    expect(INPUT_TYPES).toEqual(F.inputTypes);
    expect(CANVAS).toEqual(F.canvas);
    expect({ depth: DESIGNER_HISTORY, coalesceMs: COALESCE_MS }).toEqual(F.history);
  });

  it('says what can feed what, names things from their labels, starts and copies models', () => {
    const types = F.canFeed.map((r) => r.from);
    for (const r of F.canFeed) expect(types.filter((t) => canFeed(r.from as never, t as never)), r.from).toEqual(r.feeds);
    for (const c of F.slugs) expect(slug(c.label), JSON.stringify(c.label)).toBe(c.slug);
    const { id, ...fresh } = newModel();
    expect(id).toMatch(/^m-/);
    expect(fresh).toEqual(F.newModel);
    const source = { ...newModel(), label: F.copyLabel.label };
    const copied = copyModel(source);
    expect([copied.label, copied.id === source.id]).toEqual([F.copyLabel.copy, false]);
  });

  it('edits a model step by step, and checks it after each', () => {
    for (const seq of F.sequences) {
      const model = { ...newModel(), id: 'm-fixture' };
      seq.steps.forEach((s, i) => {
        const where = `${seq.title}, ${i + 1}. ${s.op.op}`;
        expect(plain(apply(model, s.op)), where).toEqual(s.result);
        expect(plain(model), where).toEqual(s.model);
        expect(plain(checkModel(model, lookup)), where).toEqual(s.issues);
      });
    }
  });

  it('reads a model as the designer shows it: sources, edges, order, box texts, the wire menu, its extent', () => {
    for (const m of F.models) {
      const model = m.model;
      const problems = checkModel(model, lookup);
      const first = new Map<string, string>();
      for (const p of problems) if (p.step && !first.has(p.step)) first.set(p.step, p.message);
      expect(designerStatus(problems, model.steps.length, model.inputs.length), m.title).toEqual(m.status);
      expect(plain(orderSteps(model)), m.title).toEqual(m.order);
      const edges = edgesOf(model);
      expect(plain(edges), m.title).toEqual(m.edges);
      expect(
        edges.map((e) => {
          const tool = lookup(model.steps.find((s) => s.id === e.to)!.tool);
          return edgeLabel(e.params.map((p) => tool?.parameters.find((d) => d.name === p)?.label ?? p));
        }),
        m.title,
      ).toEqual(m.edgeLabels);
      expect(model.steps.map((s) => ({ step: s.id, meta: stepMeta(s, lookup(s.tool), first.get(s.id)) })), m.title).toEqual(m.stepMeta);
      for (const c of m.sources) {
        const step = model.steps.find((s) => s.id === c.step)!;
        const p = lookup(step.tool)!.parameters.find((x) => x.name === c.param)!;
        expect(plain(sourcesFor(model, c.step, p, lookup)), `${m.title}: ${c.step}.${c.param}`).toEqual(c.options);
        expect(sourceText(model, step.values[c.param], lookup), `${m.title}: ${c.step}.${c.param}`).toBe(c.text);
      }
      for (const w of m.wires) expect(plain(connectChoices(model, w.from, w.to, lookup)), `${m.title}: ${JSON.stringify(w.from)} → ${w.to}`).toEqual(w.choices);
      expect(plain(boxesBounds(model)), m.title).toEqual(m.bounds);
    }
  });

  it('puts a new box next to the selected one, names the window, saves unnamed models, and joins typing into one undo step', () => {
    const model = F.models[4].model;
    for (const c of F.spots) expect(spotNear(c.empty ? newModel() : model, c.selected), JSON.stringify(c.selected)).toEqual(c.spot);
    for (const c of F.titles) expect(DESIGNER_TEXTS.titleOf(c.label, c.dirty)).toBe(c.title);
    for (const c of F.savedLabels) expect(savedLabel(c.label), JSON.stringify(c.label)).toBe(c.saved);
    let last: { key: string; at: number } | null = null;
    for (const c of F.joins) {
      expect(joins(c.key ?? undefined, last, c.at), JSON.stringify(c)).toBe(c.joins);
      last = c.key ? { key: c.key, at: c.at } : null;
    }
  });

  it('draws the diagram as the file says: edges, ports, fitting, zooming, the grid', () => {
    const G = F.geometry;
    for (const c of G.curves) {
      expect(plain(curve(c.a, c.b)), JSON.stringify(c)).toEqual(c.curve);
      expect(plain(curveMid(c.a, c.b)), JSON.stringify(c)).toEqual(c.mid);
    }
    expect([inputPort(G.ports.input.at), stepPort(G.ports.step.at), stepEntry(G.ports.step.at)]).toEqual([G.ports.input.port, G.ports.step.port, G.ports.step.entry]);
    for (const c of G.fits) expect(plain(fitView(c.bounds, c.width, c.height)), JSON.stringify(c)).toEqual(c.view);
    for (const c of G.zooms) expect(plain(zoomAt(c.view, c.at, c.factor)), JSON.stringify(c)).toEqual(c.result);
    for (const c of G.snaps) expect(plain(snap(c.value)), String(c.value)).toBe(c.snapped);
  });
});
