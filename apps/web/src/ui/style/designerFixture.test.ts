import { describe, expect, it } from 'vitest';
import type { Effect, EffectType, GeneratorStep, GeneratorStepKind, GeometryGenerator, Symbol } from '../../model/style';
import { EFFECT_TYPES } from '../../model/style';
import {
  addLayer,
  addParent,
  addStep,
  BUFFER_SIDES,
  childTypes,
  CLASS_OF,
  EFFECT_LABEL,
  effectsSummary,
  moveStep,
  newEffect,
  newStep,
  putEffect,
  putStep,
  removeStep,
  setEffect,
  setGeneratorKind,
  STEP_KINDS,
  STEP_LABEL,
  stepOutput,
  stepSummary,
  takesEffects,
  VERTEX_CHOICES,
  type GenClass,
  ANCHORS,
  applyPatch,
  canMove,
  canRemove,
  CAPS,
  countSuffix,
  defaultFor,
  DESIGNER_TEXTS,
  designerTitle,
  duplicateLayer,
  FONTS,
  GEOMETRIES,
  KIND_TITLE,
  LAYER_LABEL,
  LAYER_TYPES,
  layerAt,
  moveLayer,
  newDraft,
  newLayer,
  OPEN_SHAPES,
  pathOf,
  PLACEMENTS,
  POSITIONS,
  putLayer,
  removeLayer,
  RINGS,
  savedAs,
  setEnabled,
  SHAPES,
  summary,
  uid,
  UNITS,
  WAVES,
  WEIGHTS,
  ZOOM,
  zoomed,
  zoomText,
  type AnyLayer,
  type LayerPath,
  type LayerType,
  type SymbolKind,
} from './designerModel';

/**
 * The symbol designer's model as fixtures/style/v1/designer.json holds it
 * (scripts/fixtures/record-designer.test.ts): layer types and names, new
 * layers, summaries, form patches, the layer list's edits, drafts, titles
 * and the window's words. The desktop's designer checks the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/style/v1/designer.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  labels: unknown;
  layerTypes: unknown;
  kindTitles: unknown;
  geometries: unknown;
  options: Record<string, unknown>;
  newLayers: { type: LayerType; id: string; context: SymbolKind; layer: unknown }[];
  summaries: { layer: AnyLayer; summary: string }[];
  countSuffixes: [number, string][];
  patches: { layer: AnyLayer; patch: Record<string, unknown>; unset: string[]; result: unknown }[];
  uids: { ids: string[]; uid: string }[];
  edits: {
    name: string;
    symbol: Symbol;
    selected: LayerPath;
    layer: unknown;
    addParent: number | null;
    canMoveUp: boolean;
    canMoveDown: boolean;
    canRemove: boolean;
    op: { op: string; type?: LayerType; parent?: number | null; delta?: number; at?: LayerPath; on?: boolean; layer?: AnyLayer };
    result: unknown;
  }[];
  drafts: { kind: SymbolKind; path: string[] | null; draft: unknown }[];
  defaults: Record<string, unknown>;
  titles: { kind: SymbolKind; inline: string | null; dirty: boolean; title: string }[];
  saved: { name: string; path: string[]; as: unknown }[];
  paths: { text: string; path: string[] }[];
  zoom: { min: number; max: number; step: number; start: number; real: number; steps: { px: number; factor: number; result: number }[]; texts: { px: number; text: string }[] };
  texts: Record<string, unknown>;
  childTypes: { parent: GeometryGenerator; types: LayerType[] }[];
  steps: {
    labels: unknown;
    kinds: unknown;
    vertexChoices: unknown;
    bufferSides: unknown;
    classOf: unknown;
    newSteps: { kind: GeneratorStepKind; step: unknown }[];
    lines: { step: GeneratorStep; unit: string; line: string }[];
    outputs: { steps: GeneratorStep[]; input: GenClass; output: GenClass | null }[];
    edits: { name: string; layer: GeometryGenerator; op: { op: string; kind?: string; at?: number; delta?: number; step?: GeneratorStep }; result: unknown }[];
  };
  effects: {
    labels: unknown;
    types: unknown;
    newEffects: { type: EffectType; effect: unknown }[];
    takes: { path: LayerPath; takes: boolean }[];
    edits: { name: string; layer: AnyLayer; op: { op: string; type?: EffectType; on?: boolean; effect?: Effect }; result: unknown }[];
    lines: { layer: AnyLayer; line: string }[];
  };
};

const plain = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

describe('symbol designer (fixtures/style/v1/designer.json)', () => {
  it('is a v1 designer file with the names, types and options the forms use', () => {
    expect([F.format, F.version]).toEqual(['kentos.style-designer', 1]);
    expect(LAYER_LABEL).toEqual(F.labels);
    expect(LAYER_TYPES).toEqual(F.layerTypes);
    expect(KIND_TITLE).toEqual(F.kindTitles);
    expect(GEOMETRIES).toEqual(F.geometries);
    expect({ units: UNITS, shapes: SHAPES, openShapes: [...OPEN_SHAPES], placements: PLACEMENTS, anchors: ANCHORS, fonts: FONTS, weights: WEIGHTS, caps: CAPS, rings: RINGS, waves: WAVES, positions: POSITIONS }).toEqual(F.options);
  });

  it('starts new layers as the file does', () => {
    for (const c of F.newLayers) expect(plain(newLayer(c.type, c.id, c.context)), `${c.type} in ${c.context}`).toEqual(c.layer);
  });

  it('sums layers up as the file does', () => {
    for (const c of F.summaries) expect(summary(c.layer), JSON.stringify(c.layer)).toBe(c.summary);
    for (const [n, s] of F.countSuffixes) expect(countSuffix(n), String(n)).toBe(s);
  });

  it('turns form patches into layers as the file does', () => {
    for (const c of F.patches) {
      const patch = { ...c.patch, ...Object.fromEntries(c.unset.map((k) => [k, undefined])) };
      expect(plain(applyPatch(c.layer, patch)), JSON.stringify(c.patch)).toEqual(c.result);
    }
  });

  it('gives new layers the ids the file does', () => {
    for (const c of F.uids) expect(uid(c.ids.map((id) => ({ id, type: 'simpleFill', color: 'ink' }))), c.ids.join(',')).toBe(c.uid);
  });

  for (const e of F.edits)
    it(`edits the layer list: ${e.name}`, () => {
      expect(plain(layerAt(e.symbol, e.selected) ?? null)).toEqual(e.layer);
      expect(addParent(e.symbol, e.selected)).toBe(e.addParent);
      expect([canMove(e.symbol, e.selected, -1), canMove(e.symbol, e.selected, 1), canRemove(e.symbol, e.selected)]).toEqual([e.canMoveUp, e.canMoveDown, e.canRemove]);
      const o = e.op;
      const out =
        o.op === 'add'
          ? addLayer(e.symbol, o.type!, o.parent ?? null)
          : o.op === 'move'
            ? moveLayer(e.symbol, e.selected, o.delta!)
            : o.op === 'duplicate'
              ? duplicateLayer(e.symbol, e.selected)
              : o.op === 'remove'
                ? removeLayer(e.symbol, e.selected)
                : o.op === 'enable'
                  ? { symbol: setEnabled(e.symbol, o.at!, o.on!), selected: e.selected }
                  : { symbol: putLayer(e.symbol, o.at!, o.layer!), selected: e.selected };
      expect(plain(out)).toEqual(e.result);
    });

  it('starts drafts, slots, titles and saved names as the file does', () => {
    for (const d of F.drafts) expect(plain(newDraft(d.kind, d.path ?? undefined))).toEqual(d.draft);
    expect({ fill: defaultFor('fill'), line: defaultFor('line'), marker: defaultFor('marker') }).toEqual(F.defaults);
    for (const t of F.titles) expect(designerTitle(t.kind, t.inline, t.dirty)).toBe(t.title);
    for (const s of F.saved) expect(savedAs({ name: s.name, path: s.path, symbol: { type: 'line', layers: [] } })).toEqual(s.as);
    for (const p of F.paths) expect(pathOf(p.text)).toEqual(p.path);
  });

  it('scales the preview as the file does', () => {
    const { steps, texts, ...zoom } = F.zoom;
    expect(ZOOM).toEqual(zoom);
    for (const s of steps) expect(zoomed(s.px, s.factor)).toBe(s.result);
    for (const t of texts) expect(zoomText(t.px)).toBe(t.text);
  });

  it('says what the file says', () => {
    const T = F.texts as Record<string, never>;
    const fixed = Object.fromEntries(Object.entries(DESIGNER_TEXTS).filter(([, v]) => typeof v === 'string'));
    expect(fixed).toEqual(Object.fromEntries(Object.keys(fixed).map((k) => [k, T[k]])));
    const into = F.texts.intoMarker as { parent: string; text: string };
    expect(DESIGNER_TEXTS.intoMarker(into.parent)).toBe(into.text);
    const inMarker = F.texts.inMarker as { parent: string; text: string };
    expect(DESIGNER_TEXTS.inMarker(inMarker.parent)).toBe(inMarker.text);
    for (const n of F.texts.notSaved as { first: string; more: number; text: string }[]) expect(DESIGNER_TEXTS.notSaved(n.first, n.more)).toBe(n.text);
    const saved = F.texts.saved as { name: string; text: string };
    expect(DESIGNER_TEXTS.saved(saved.name)).toBe(saved.text);
    const intoGenerator = F.texts.intoGenerator as { parent: string; text: string };
    expect(DESIGNER_TEXTS.intoGenerator(intoGenerator.parent)).toBe(intoGenerator.text);
    const inGenerator = F.texts.inGenerator as { parent: string; text: string };
    expect(DESIGNER_TEXTS.inGenerator(inGenerator.parent)).toBe(inGenerator.text);
  });

  it('lists a parent’s child types and a generator’s steps as the file does (docs/adr/0226 §1)', () => {
    for (const c of F.childTypes) expect(childTypes(c.parent), JSON.stringify(c.parent)).toEqual(c.types);
    const S = F.steps;
    expect([STEP_LABEL, STEP_KINDS, VERTEX_CHOICES, BUFFER_SIDES, CLASS_OF]).toEqual([S.labels, S.kinds, S.vertexChoices, S.bufferSides, S.classOf]);
    for (const c of S.newSteps) expect(newStep(c.kind), c.kind).toEqual(c.step);
    for (const c of S.lines) expect(stepSummary(c.step, c.unit), JSON.stringify(c.step)).toBe(c.line);
    for (const c of S.outputs) expect(stepOutput(c.steps, c.input), `${JSON.stringify(c.steps)} on ${c.input}`).toBe(c.output);
    for (const e of S.edits) {
      const o = e.op;
      const out =
        o.op === 'add'
          ? addStep(e.layer, o.kind as GeneratorStepKind)
          : o.op === 'move'
            ? moveStep(e.layer, o.at!, o.delta!)
            : o.op === 'remove'
              ? removeStep(e.layer, o.at!)
              : o.op === 'put'
                ? putStep(e.layer, o.at!, o.step!)
                : setGeneratorKind(e.layer, o.kind as SymbolKind);
      expect(plain(out), e.name).toEqual(e.result);
    }
  });

  it('switches and sums effects up as the file does (docs/adr/0226 §2)', () => {
    const E = F.effects;
    expect([EFFECT_LABEL, EFFECT_TYPES]).toEqual([E.labels, E.types]);
    for (const c of E.newEffects) expect(newEffect(c.type), c.type).toEqual(c.effect);
    for (const c of E.takes) expect(takesEffects(c.path)).toBe(c.takes);
    for (const e of E.edits) {
      const o = e.op;
      const out = o.op === 'set' ? setEffect(e.layer, o.type!, o.on!) : putEffect(e.layer, o.effect!);
      expect(plain(out), e.name).toEqual(e.result);
    }
    for (const c of E.lines) expect(effectsSummary(c.layer)).toBe(c.line);
  });
});
