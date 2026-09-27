import { stepName, type ModelIssue, type ModelStep, type ProcessingModel, type ValueSource } from '../../../processing/model';
import { edgesOf, sourcesFor } from '../../../processing/modelEdit';
import type { ProcessingTool } from '../../../processing/types';

/**
 * The model designer's words and rules apart from the DOM (ModelDesigner,
 * ModelCanvas, modelPalette, modelInspector; docs/specs/model-designer.md):
 * the title and the status line, what a box and an edge say, the menu a
 * wire dropped on a step opens, how a source is named, where a new box
 * goes, when typing joins one undo step, and the diagram's geometry (box
 * sizes, the curve of an edge, fitting, zooming, the grid). The model's own
 * edits are processing/modelEdit.ts's. fixtures/processing/v1/designer.json
 * holds both for the desktop (format in fixtures/processing/README.md).
 */

type Lookup = (id: string) => ProcessingTool | undefined;
export type Pt = { x: number; y: number };
export type NodeRef = { kind: 'input'; name: string } | { kind: 'step'; id: string };

export const DESIGNER_TEXTS = {
  title: 'Model tasarımcısı',
  titleOf: (label: string, dirty: boolean) => `Model tasarımcısı: ${label || 'adsız'}${dirty ? ' •' : ''}`,
  notFound: (id: string) => `Model bulunamadı: ${id}.`,
  footer: { layout: 'Düzenle', layoutTip: 'Kutuları bağlantı sırasına göre sütunlara dizer', close: 'Kapat', saveRun: 'Kaydet ve çalıştır…', save: 'Kaydet' },
  status: {
    problems: (n: number, first: string) => `${n} sorun var; model kaydedilebilir ama çalışmaz. ${first}`,
    ready: (steps: number, inputs: number) => `${steps} adım, ${inputs} girdi. Model çalışmaya hazır.`,
    empty: 'Soldan bir girdi ve bir araç ekleyerek başlayın.',
  },
  save: {
    unnamed: 'Adsız model',
    saved: (label: string, problems: number) => `“${label}” modeli kaydedildi${problems ? `; ${problems} sorun giderilene kadar çalışmaz` : ''}.`,
  },
  unsaved: { after: 'Pencere kapanırsa bu değişiklikler kaybolur.', verb: 'kapat' },
  remove: {
    title: 'Modeli sil',
    question: (label: string) => `“${label}” modeli silinsin mi? Bu geri alınamaz.`,
    action: 'Modeli sil',
    done: (label: string) => `“${label}” modeli silindi.`,
  },
  pick: { point: 'Nokta', command: (label: string) => `Model tasarımcısı: ${label}`, unnamed: 'nokta' },
  connect: {
    header: (step: string) => `${step}: hangi girdi?`,
    fromStep: (output: string, param: string) => `${output} → ${param}`,
    replaces: 'Mevcut bağlantının yerine geçer',
    none: (what: string) => `“${what}” bu adımın hiçbir girdisine uymuyor`,
  },
  canvas: {
    label: 'Model diyagramı',
    zoomOut: 'Uzaklaş',
    zoomIn: 'Yakınlaş',
    fit: 'Tümünü göster (çift tık)',
    port: 'Sürükleyip bir adımın üzerine bırakın',
    inputMeta: (type: string, optional: boolean) => `Girdi: ${type}${optional ? ', isteğe bağlı' : ''}`,
    links: (n: number) => `${n} bağlantı`,
    noLinks: 'Bağlantı yok',
  },
  palette: {
    label: 'Model parçaları',
    inputs: 'Girdi ekle',
    tools: 'Araçlar',
    search: 'Araç ara',
    empty: 'Aramayla eşleşen araç yok.',
    toolNote: 'Tıklayın ya da tuvale sürükleyin',
    tip: 'Bir aracı tıklayın ya da tuvale sürükleyin. Seçili kutu varsa yeni adım ona bağlanır; bağlantıyı değiştirmek için kutunun sağındaki noktadan sürükleyin.',
  },
  inspector: {
    label: 'Seçilen kutunun ayarları',
    model: {
      kind: 'Model',
      lead: 'Adı ve açıklaması araç kutusunda ve menüde görünür.',
      name: 'Ad',
      nameAria: 'Model adı',
      category: 'Kategori',
      description: 'Açıklama',
      descriptionAria: 'Model açıklaması',
      descriptionHint: 'Ne yapar, tek cümle',
      outputs: 'Model çıktıları',
      output: (step: string, output: string) => `${step} › ${output}`,
      noStep: 'adım yok',
      removeOutput: (label: string) => `“${label}” çıktısını kaldır`,
      addOutput: 'Çıktı ekle',
      noOutput: 'Eklenebilecek çıktı yok',
      remove: 'Modeli sil',
    },
    problems: {
      title: 'Sorunlar',
      titleCount: (n: number) => `Sorunlar (${n})`,
      ready: 'Model çalışmaya hazır.',
      start: 'Başlamak için soldan bir girdi ve bir araç ekleyin.',
      ofStep: (step: string, message: string) => `${step}: ${message}`,
    },
    input: {
      kind: (type: string) => `Girdi: ${type}`,
      lead: 'Model çalıştırılırken kullanıcıdan istenir.',
      label: 'Etiket',
      labelAria: 'Girdi etiketi',
      variable: (name: string) => `Değişken adı: ${name}`,
      description: 'Açıklama',
      descriptionAria: 'Girdi açıklaması',
      descriptionHint: 'Pencerede etiketin altında görünür',
      optional: 'İsteğe bağlı',
      default: 'Varsayılan',
      scopeAria: 'Varsayılan kapsam',
      scopes: { selection: 'Seçili', visible: 'Görünen', all: 'Tümü' },
      kinds: 'Uygun nesneler',
      kindsNote: 'Hiçbiri seçili değilse her tür alınır; adımlar kendi türlerini ayrıca süzer.',
      min: 'En az',
      max: 'En çok',
      none: 'yok',
      integer: 'Tam sayı',
      textDefault: 'Varsayılan metin',
      allowEmpty: 'Boş bırakılabilir',
      newLayer: 'Varsayılan yeni katman',
      newLayerAria: 'Varsayılan yeni katman adı',
      newLayerNote: 'Çalıştırırken var olan bir katman da seçilebilir.',
      pointNote: 'Çalıştırırken haritada gösterilir ya da Y,X yazılır.',
      users: 'Kullanan adımlar',
      noUsers: 'Henüz hiçbir adım bu girdiyi kullanmıyor. Kutunun sağındaki noktadan bir adıma sürükleyin.',
      remove: 'Girdiyi sil',
    },
    step: {
      unknown: 'Bilinmeyen araç',
      unknownNote: (tool: string) => `“${tool}” bu sürümde yok. Adımı silin ya da aracı sağlayan eklentiyi yükleyin.`,
      caption: 'Başlık',
      captionAria: 'Adım başlığı',
      captionNote: 'Diyagramda ve iletilerde görünür.',
      params: 'Parametreler',
      advanced: (n: number) => `Gelişmiş (${n})`,
      optional: 'isteğe bağlı',
      sourceAria: (label: string) => `${label}: kaynak`,
      toolDefault: 'Aracın varsayılanı',
      fixed: 'Sabit değer',
      modelInput: 'Model girdisi',
      asInput: 'Yeni model girdisi yap',
      asInputNote: 'Model çalıştırılırken bu değer sorulur',
      inputSource: (label: string) => `Girdi: ${label}`,
      outputSource: (step: string, output: string) => `${step} › ${output}`,
      remove: 'Adımı sil',
    },
  },
} as const;

const T = DESIGNER_TEXTS;

/** The line under the diagram: the problems and the first one, or how big the model is and that it is ready, or how to begin. */
export function designerStatus(problems: readonly ModelIssue[], steps: number, inputs: number): { kind: 'warn' | 'ok'; text: string } {
  if (problems.length) return { kind: 'warn', text: T.status.problems(problems.length, problems[0].message) };
  return { kind: 'ok', text: steps ? T.status.ready(steps, inputs) : T.status.empty };
}

/** The name a model is saved under: its label, or “Adsız model” when that is blank. */
export const savedLabel = (label: string): string => (label.trim() ? label : T.save.unnamed);

/** How a step parameter's source reads on its dropdown. */
export function sourceText(model: ProcessingModel, src: ValueSource | undefined, lookup: Lookup): string {
  if (!src) return T.inspector.step.toolDefault;
  if (src.kind === 'value') return T.inspector.step.fixed;
  if (src.kind === 'input') return T.inspector.step.inputSource(model.inputs.find((i) => i.name === src.name)?.label ?? src.name);
  const step = model.steps.find((s) => s.id === src.step);
  const out = step && lookup(step.tool)?.outputs?.find((o) => o.name === src.output);
  return T.inspector.step.outputSource(step ? stepName(step, lookup) : src.step, out?.label ?? src.output);
}

/** A step box's second line: its first problem, the tool's name under a caption, how many links, or none. */
export function stepMeta(step: ModelStep, tool: ProcessingTool | undefined, problem: string | undefined): { warn: string } | { text: string } {
  if (problem) return { warn: problem };
  if (step.caption && tool) return { text: tool.label };
  const links = Object.values(step.values).filter((v) => v.kind !== 'value').length;
  return { text: links ? T.canvas.links(links) : T.canvas.noLinks };
}

/** An edge's label: the parameter it feeds, or the first and how many more. */
export const edgeLabel = (labels: readonly string[]): string => (labels.length > 1 ? `${labels[0]} +${labels.length - 1}` : (labels[0] ?? ''));

export interface ConnectChoice {
  /** The parameter it feeds. */
  param: string;
  label: string;
  /** Said when the parameter is fed by something else now. */
  detail?: string;
  /** The parameter is fed by exactly this already. */
  checked: boolean;
  src: ValueSource;
}

/**
 * A wire from a box's port dropped on a step: which of the step's
 * parameters the source can feed, each output of a source step in turn
 * (never a cycle, never a type that does not fit), or the one line that
 * says nothing fits.
 */
export function connectChoices(model: ProcessingModel, from: NodeRef, toStep: string, lookup: Lookup): { header: string; items: ConnectChoice[]; none: string | null } | null {
  const step = model.steps.find((s) => s.id === toStep);
  const tool = step && lookup(step.tool);
  if (!step || !tool) return null;
  const sourceStep = from.kind === 'step' ? model.steps.find((s) => s.id === from.id) : undefined;
  const outputs = from.kind === 'input' ? [null] : (lookup(sourceStep?.tool ?? '')?.outputs ?? []);
  const items: ConnectChoice[] = [];
  for (const out of outputs) {
    for (const p of tool.parameters) {
      const fits = sourcesFor(model, toStep, p, lookup).find((o) =>
        from.kind === 'input' ? o.src.kind === 'input' && o.src.name === from.name : o.src.kind === 'output' && o.src.step === from.id && o.src.output === out?.name,
      );
      if (!fits) continue;
      const current = step.values[p.name];
      const same = JSON.stringify(current) === JSON.stringify(fits.src);
      items.push({ param: p.name, label: out ? T.connect.fromStep(out.label, p.label) : p.label, ...(current && !same ? { detail: T.connect.replaces } : {}), checked: same, src: fits.src });
    }
  }
  const what = from.kind === 'input' ? model.inputs.find((i) => i.name === from.name)?.label : sourceStep && stepName(sourceStep, lookup);
  return { header: T.connect.header(stepName(step, lookup)), items, none: items.length ? null : T.connect.none(String(what)) };
}

/** Where a new box goes: to the right of the selected one, else at the left below every other box. */
export function spotNear(model: ProcessingModel, selected: NodeRef | null): Pt {
  const pos = selected?.kind === 'input' ? model.inputPositions?.[selected.name] : selected?.kind === 'step' ? model.steps.find((s) => s.id === selected.id)?.position : undefined;
  if (pos) return { x: pos.x + CANVAS.column, y: pos.y };
  const ys = [...model.steps.map((s) => s.position?.y ?? 0), ...Object.values(model.inputPositions ?? {}).map((p) => p.y)];
  return { x: 40, y: ys.length ? Math.max(...ys) + CANVAS.row : 40 };
}

/** The designer's own undo: this many steps are kept; typing into one field within COALESCE_MS is one step. */
export const DESIGNER_HISTORY = 100;
export const COALESCE_MS = 1200;

/** Whether a change joins the undo step before it: the same field, typed into again within COALESCE_MS. */
export function joins(key: string | undefined, last: { key: string; at: number } | null, now: number): boolean {
  return !!key && last?.key === key && now - last.at < COALESCE_MS;
}

// ── The diagram's geometry ────────────────────────────────────────────

export const CANVAS = {
  inputW: 190,
  inputH: 52,
  stepW: 240,
  stepH: 60,
  /** Boxes snap to this (px); the background grid is twice it. */
  grid: 10,
  zoomMin: 0.35,
  zoomMax: 2,
  /** The zoom buttons' factor. */
  zoomStep: 1.25,
  /** The wheel zooms by exp(−deltaY × wheel). */
  wheel: 0.0015,
  /** Fitting leaves this margin (px) and never zooms in past 1:1. */
  fitPad: 48,
  /** The view of an empty diagram. */
  empty: { x: 24, y: 24, k: 1 },
  /** A press becomes a drag after this many px; a tool carried from the palette after `paletteDrag`. */
  drag: 3,
  paletteDrag: 5,
  /** Next to a box, and below the lowest one (spotNear, autoLayout's columns and rows). */
  column: 290,
  row: 100,
  /** An edge's control points stand at least this far out. */
  bend: 40,
  /** Edge labels at their target: they end this far left of the step's entry point, the first row's baseline this far
   * above it, and each further row this much higher. */
  labelGap: 8,
  labelRise: 6,
  labelRow: 13,
} as const;

export type View = { x: number; y: number; k: number };

/** A world position snapped to the grid. */
export const snap = (v: number): number => Math.round(v / CANVAS.grid) * CANVAS.grid;

/** An edge from a port (a box's right side) to a step's left side: a cubic whose control points stand out horizontally. */
export function curve(a: Pt, b: Pt): { c1: Pt; c2: Pt } {
  const dx = Math.max(CANVAS.bend, Math.abs(b.x - a.x) / 2);
  return { c1: { x: a.x + dx, y: a.y }, c2: { x: b.x - dx, y: b.y } };
}

/** Where an input's port is (its right side, halfway down). */
export const inputPort = (at: Pt): Pt => ({ x: at.x + CANVAS.inputW, y: at.y + CANVAS.inputH / 2 });
/** Where a step's port is (its right side, halfway down; only a step whose tool has outputs shows one). */
export const stepPort = (at: Pt): Pt => ({ x: at.x + CANVAS.stepW, y: at.y + CANVAS.stepH / 2 });
/** Where an edge ends on a step (its left side, halfway down). */
export const stepEntry = (at: Pt): Pt => ({ x: at.x, y: at.y + CANVAS.stepH / 2 });

/** An edge's label: its text, the tip naming every parameter it feeds, and where it stands. */
export interface EdgeLabel {
  from: NodeRef;
  to: string;
  text: string;
  title: string;
  /** The label's right end on its baseline (world px). */
  at: Pt;
}

/**
 * Every edge's label, at its target step: right-aligned, ending `labelGap`
 * px left of the step's entry point, the first row's baseline `labelRise`
 * px above it, each further edge into the step one row (`labelRow` px)
 * higher. The edge whose source port is lowest on the diagram takes the row
 * nearest the entry (ties: the edges' order), so read top to bottom the
 * labels follow their sources top to bottom. An edge into a step that is not
 * in the model has none.
 */
export function edgeLabels(model: ProcessingModel, lookup: Lookup): EdgeLabel[] {
  const edges = edgesOf(model);
  const sourceY = (from: NodeRef) =>
    from.kind === 'input' ? inputPort(model.inputPositions?.[from.name] ?? { x: 40, y: 40 }).y : stepPort(model.steps.find((s) => s.id === from.id)?.position ?? { x: 0, y: 0 }).y;
  const rows = new Map<number, number>();
  const into = new Map<string, number[]>();
  edges.forEach((e, i) => into.set(e.to, [...(into.get(e.to) ?? []), i]));
  for (const indices of into.values()) [...indices].sort((a, b) => sourceY(edges[b].from) - sourceY(edges[a].from) || a - b).forEach((i, row) => rows.set(i, row));
  return edges.flatMap((e, i) => {
    const step = model.steps.find((s) => s.id === e.to);
    if (!step) return [];
    const tool = lookup(step.tool);
    const names = e.params.map((p) => tool?.parameters.find((d) => d.name === p)?.label ?? p);
    const entry = stepEntry(step.position ?? { x: 0, y: 0 });
    const at = { x: entry.x - CANVAS.labelGap, y: entry.y - CANVAS.labelRise - (rows.get(i) ?? 0) * CANVAS.labelRow };
    return [{ from: e.from, to: e.to, text: edgeLabel(names), title: names.join(', '), at }];
  });
}

/** Every box's extent (an input without a place at 40,40; a step without one at 0,0), or null for an empty model. */
export function boxesBounds(model: ProcessingModel): { x: number; y: number; w: number; h: number } | null {
  if (!model.inputs.length && !model.steps.length) return null;
  const boxes = [
    ...model.inputs.map((i) => ({ ...(model.inputPositions?.[i.name] ?? { x: 40, y: 40 }), w: CANVAS.inputW, h: CANVAS.inputH })),
    ...model.steps.map((s) => ({ ...(s.position ?? { x: 0, y: 0 }), w: CANVAS.stepW, h: CANVAS.stepH })),
  ];
  const x = Math.min(...boxes.map((b) => b.x));
  const y = Math.min(...boxes.map((b) => b.y));
  return { x, y, w: Math.max(...boxes.map((b) => b.x + b.w)) - x, h: Math.max(...boxes.map((b) => b.y + b.h)) - y };
}

/** The view that frames every box in a canvas `width` × `height`, centred; it zooms out only, never past 1:1. */
export function fitView(bounds: { x: number; y: number; w: number; h: number } | null, width: number, height: number): View {
  if (!bounds || !width) return { ...CANVAS.empty };
  const pad = CANVAS.fitPad;
  const k = Math.min(1, (width - pad * 2) / Math.max(1, bounds.w), (height - pad * 2) / Math.max(1, bounds.h));
  return { k, x: (width - bounds.w * k) / 2 - bounds.x * k, y: (height - bounds.h * k) / 2 - bounds.y * k };
}

/** The view zoomed by `f` about a canvas point, which stays over the same world point; the scale stays within its limits. */
export function zoomAt(view: View, p: Pt, f: number): View {
  const k = Math.min(CANVAS.zoomMax, Math.max(CANVAS.zoomMin, view.k * f));
  const wx = (p.x - view.x) / view.k;
  const wy = (p.y - view.y) / view.k;
  return { k, x: p.x - wx * k, y: p.y - wy * k };
}
