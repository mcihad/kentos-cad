import { alignMoves, copiesOf, distributeMoves, invertible, mirrorMatrix, polarArray, rectArray, restack, transformMoves, unitsOf, type AlignSide, type AlignTo, type Anchor, type Distribute, type PolarArraySpec, type RectArraySpec, type TransformSpec } from '../../style/svg/arrange';
import { alignNodes, breakAtNodes, cornerNodes, deleteNodes, deleteSegments, distributeNodes, insertMidNodes, joinEnds, segmentsTo, setNodeType, type NodeRef, type NodeType } from '../../style/svg/nodeOps';
import type { Matrix, Pt, SubPath } from '../../style/svg/pathData';
import { applyResult, booleanShapes, breakApart, closeShapes, combineShapes, cutShapes, offsetShapes, openShapes, reverseShapes, shapesToPath, simplifyShapes, strokeToPath, type OpResult } from '../../style/svg/pathOps';
import type { Join } from '../../style/svg/pathStroke';
import { shapesBox, transformShape, type SvgDoc, type SvgShape } from '../../style/svg/svgModel';
import type { SvgCanvas } from './svgCanvas';
import type { CanvasOptions, ToolId } from './svgView';

/**
 * What the SVG editor's menus, panels and keys do to the drawing: path
 * operations, node operations, align and distribute, the numeric
 * transforms, arrays, stacking order and selection helpers. Each is one
 * undo step and says what happened in the status line. The panels' own
 * settings (the open tab, distances, array counts) live here too, so they
 * survive the panels being redrawn.
 */

export interface ActionsHost {
  doc: SvgDoc;
  readonly selection: ReadonlySet<string>;
  readonly options: CanvasOptions;
  readonly canvas: SvgCanvas;
  readonly nodeEdit: string | null;
  change(key: string, fn: () => void): void;
  select(ids: string[]): void;
  status(text: string, kind?: 'ok' | 'warn'): void;
  refresh(): void;
  editNodes(id: string | null): void;
  setTool(t: ToolId): void;
}

export type PathOpId =
  | 'union'
  | 'difference'
  | 'intersection'
  | 'exclusion'
  | 'division'
  | 'cut'
  | 'combine'
  | 'breakApart'
  | 'split'
  | 'toPath'
  | 'strokeToPath'
  | 'inset'
  | 'outset'
  | 'simplify'
  | 'reverse'
  | 'close'
  | 'open';

export type Tab = 'props' | 'align' | 'transform' | 'array';

/** The panels' settings, kept while the editor is open. */
export interface PanelState {
  tab: Tab;
  alignTo: AlignTo;
  alignAsOne: boolean;
  offset: number;
  offsetJoin: Join;
  simplify: number;
  corner: number;
  transform: {
    kind: TransformSpec['kind'];
    relative: boolean;
    x: number;
    y: number;
    sx: number;
    sy: number;
    lock: boolean;
    deg: number;
    ccw: boolean;
    about: Anchor | 'point';
    point: Pt | null;
    anchor: Anchor;
    ax: number;
    ay: number;
    m: [number, number, number, number, number, number];
    separately: boolean;
  };
  array: {
    kind: 'rect' | 'polar' | 'mirror';
    rect: RectArraySpec;
    polar: Omit<PolarArraySpec, 'centre'> & { at: 'box' | 'canvas' | 'point'; point: Pt | null };
    mirror: { axis: 'v' | 'h' | 'angle'; deg: number; at: 'box' | 'canvas' | 'point'; point: Pt | null };
    preview: boolean;
  };
}

export class EditActions {
  private readonly host: ActionsHost;
  private seq = 0;
  readonly ui: PanelState;

  constructor(host: ActionsHost) {
    this.host = host;
    const unit = Math.max(0.1, Math.round((host.doc.width / 50) * 100) / 100);
    this.ui = {
      tab: 'props',
      alignTo: 'selection',
      alignAsOne: false,
      offset: unit,
      offsetJoin: 'round',
      simplify: 0.2,
      corner: unit,
      transform: { kind: 'move', relative: true, x: 0, y: 0, sx: 100, sy: 100, lock: true, deg: 90, ccw: true, about: 'c', point: null, anchor: 'c', ax: 0, ay: 0, m: [1, 0, 0, 1, 0, 0], separately: false },
      array: {
        kind: 'rect',
        rect: { rows: 2, cols: 3, dx: unit, dy: unit, mode: 'gap' },
        polar: { count: 6, angle: 360, rotate: true, ccw: true, at: 'canvas', point: null },
        mirror: { axis: 'v', deg: 45, at: 'box', point: null },
        preview: true,
      },
    };
  }

  /** The chosen shapes back to front (as the list stacks them). */
  get chosen(): SvgShape[] {
    return this.host.doc.shapes.filter((s) => this.host.selection.has(s.id));
  }

  /** One undo step, then everything redrawn. */
  private run(label: string, fn: () => void): void {
    this.host.change(`${label}#${++this.seq}`, fn);
    this.host.refresh();
  }

  private needs(n: number, what: string): boolean {
    if (this.chosen.length >= n) return true;
    this.host.status(what, 'warn');
    return false;
  }

  // ── Path operations ──────────────────────────────────────────────────

  path(op: PathOpId): void {
    const sel = this.chosen;
    if (!sel.length) return this.host.status('Önce şekil seçin.', 'warn');
    const size = shapesBox(sel);
    const extent = size ? Math.max(size.maxX - size.minX, size.maxY - size.minY) : 1;
    let r: OpResult;
    switch (op) {
      case 'union':
      case 'difference':
      case 'intersection':
      case 'exclusion':
      case 'division':
        r = booleanShapes(op, sel);
        break;
      case 'cut':
        r = cutShapes(sel);
        break;
      case 'combine':
        r = combineShapes(sel);
        break;
      case 'breakApart':
      case 'split': {
        const parts = sel.map((s) => breakApart(s, op === 'split'));
        const ok = parts.filter((p): p is Extract<OpResult, { add: SvgShape[] }> => !('error' in p));
        r = ok.length ? { add: ok.flatMap((p) => p.add), remove: ok.flatMap((p) => p.remove) } : parts[0];
        break;
      }
      case 'toPath':
        r = shapesToPath(sel);
        break;
      case 'strokeToPath':
        r = strokeToPath(sel);
        break;
      case 'inset':
      case 'outset':
        r = offsetShapes(sel, op === 'inset' ? -this.ui.offset : this.ui.offset, this.ui.offsetJoin);
        break;
      case 'simplify':
        r = simplifyShapes(sel, (extent * this.ui.simplify) / 100);
        break;
      case 'reverse':
        r = reverseShapes(sel);
        break;
      case 'close':
        r = closeShapes(sel);
        break;
      case 'open':
        r = openShapes(sel);
        break;
    }
    if ('error' in r) return this.host.status(r.error, 'warn');
    const res = r;
    this.run(PATH_LABEL[op], () => {
      this.host.doc.shapes = applyResult(this.host.doc.shapes, res);
      if (this.host.nodeEdit && res.remove.includes(this.host.nodeEdit)) this.host.editNodes(null);
    });
    this.host.select(res.add.length ? res.add.map((s) => s.id) : []);
    this.host.status(`${PATH_LABEL[op]}${res.note ? `: ${res.note}` : '.'}`);
  }

  // ── Nodes ────────────────────────────────────────────────────────────

  private get nodeTool() {
    return this.host.canvas.nodes;
  }

  private nodes(label: string, fn: (subs: SubPath[], refs: NodeRef[]) => { subs: SubPath[]; refs?: NodeRef[] } | { error: string }, need = 1): void {
    const t = this.nodeTool;
    const s = t.shape;
    if (!s) return this.host.status('Önce bir yolun düğümlerini düzenlemeye başlayın (Düğüm aracı, A).', 'warn');
    const refs = t.selected;
    if (refs.length < need) return this.host.status(need > 1 ? `Bu işlem için en az ${need} düğüm seçin (Shift ile ekleyin ya da kutu çizin).` : 'Önce düğüm seçin: düğüme tıklayın ya da boşlukta sürükleyip kutu çizin.', 'warn');
    const r = fn(s.subs, refs);
    if ('error' in r) return this.host.status(r.error, 'warn');
    t.apply(label, r.subs, r.refs ?? refs);
    this.host.status(`${label}.`);
  }

  nodeType(type: NodeType): void {
    this.nodes(`Düğüm türü: ${NODE_TYPE[type]}`, (subs, refs) => ({ subs: setNodeType(subs, refs, type) }));
  }
  nodeInsert(): void {
    this.nodes('Düğüm ekle', (subs, refs) => {
      const r = insertMidNodes(subs, refs);
      return r.refs.length > refs.length ? r : { error: 'Araya düğüm eklemek için bir parçanın iki ucunu seçin.' };
    }, 2);
  }
  nodeDelete(keepShape: boolean): void {
    this.nodes('Düğümü sil', (subs, refs) => ({ subs: deleteNodes(subs, refs, keepShape), refs: [] }));
  }
  nodeJoin(merge: boolean): void {
    this.nodes(merge ? 'Uçları birleştir' : 'Uçları parçayla birleştir', (subs, refs) => joinEnds(subs, refs, merge), 2);
  }
  nodeBreak(): void {
    this.nodes('Düğümde kır', (subs, refs) => ({ subs: breakAtNodes(subs, refs), refs: [] }));
  }
  nodeDeleteSegment(): void {
    this.nodes('Parçayı sil', (subs, refs) => {
      const r = deleteSegments(subs, refs);
      return Array.isArray(r) ? { subs: r, refs: [] } : r;
    }, 2);
  }
  nodeSegments(kind: 'line' | 'curve'): void {
    this.nodes(kind === 'line' ? 'Parçalar düz' : 'Parçalar eğri', (subs, refs) => ({ subs: segmentsTo(subs, refs, kind) }), 2);
  }
  nodeCorner(mode: 'fillet' | 'chamfer'): void {
    this.nodes(mode === 'fillet' ? `Köşe yuvarla (R ${this.ui.corner})` : `Pah kır (${this.ui.corner})`, (subs, refs) => cornerNodes(subs, refs, mode, this.ui.corner));
  }
  nodeAlign(axis: 'x' | 'y', to: 'min' | 'mid' | 'max'): void {
    this.nodes('Düğümleri hizala', (subs, refs) => ({ subs: alignNodes(subs, refs, axis, to) }), 2);
  }
  nodeDistribute(axis: 'x' | 'y'): void {
    this.nodes('Düğümleri dağıt', (subs, refs) => ({ subs: distributeNodes(subs, refs, axis) }), 3);
  }
  /** Arrow keys move the chosen nodes. */
  nodeNudge(dx: number, dy: number): boolean {
    const t = this.nodeTool;
    if (!t.shape || !t.selected.length) return false;
    this.nodes('Düğümü taşı', (subs, refs) => ({ subs: subs.map((sp, si) => ({ closed: sp.closed, nodes: sp.nodes.map((n, ni) => (refs.some((r) => r.sub === si && r.index === ni) ? { ...n, x: n.x + dx, y: n.y + dy, ...(n.in ? { in: [n.in[0] + dx, n.in[1] + dy] as Pt } : {}), ...(n.out ? { out: [n.out[0] + dx, n.out[1] + dy] as Pt } : {}) } : n)) })) }));
    return true;
  }
  selectAllNodes(): void {
    const s = this.nodeTool.shape;
    if (!s) return;
    this.nodeTool.setSelected(s.subs.flatMap((sp, si) => sp.nodes.map((_, ni) => ({ sub: si, index: ni }))));
    this.host.refresh();
  }

  // ── Arranging ────────────────────────────────────────────────────────

  /** Matrices per unit applied to the unit's shapes. */
  private moveUnits(label: string, per: (units: ReturnType<typeof unitsOf>) => Matrix[]): void {
    const units = unitsOf(this.host.doc.shapes, [...this.host.selection]);
    if (units.some((u) => u.ids.some((id) => this.host.doc.shapes.find((s) => s.id === id)?.locked))) return this.host.status('Seçimde kilitli şekil var: önce kilidini açın (şekil listesinde kilit düğmesi).', 'warn');
    const ms = per(units);
    const byId = new Map<string, Matrix>();
    units.forEach((u, i) => u.ids.forEach((id) => byId.set(id, ms[i])));
    this.run(label, () => (this.host.doc.shapes = this.host.doc.shapes.map((s) => (byId.has(s.id) ? transformShape(s, byId.get(s.id)!) : s))));
  }

  align(side: AlignSide): void {
    if (!this.needs(1, 'Hizalamak için şekil seçin.')) return;
    const one = unitsOf(this.host.doc.shapes, [...this.host.selection]).length < 2;
    this.moveUnits('Hizala', (units) => alignMoves(units, side, this.ui.alignTo, this.host.doc, this.ui.alignAsOne));
    this.host.status(one && this.ui.alignTo !== 'canvas' ? 'Tek şekil tuvale hizalandı.' : 'Hizalandı.');
  }

  distribute(how: Distribute): void {
    if (unitsOf(this.host.doc.shapes, [...this.host.selection]).length < 3) return this.host.status('Dağıtmak için en az üç şekil (ya da grup) seçin.', 'warn');
    this.moveUnits('Dağıt', (units) => distributeMoves(units, how));
  }

  transform(spec: TransformSpec, separately: boolean): void {
    if (!this.needs(1, 'Dönüştürmek için şekil seçin.')) return;
    const ms = transformMoves(unitsOf(this.host.doc.shapes, [...this.host.selection]), spec, separately);
    if (!ms.every(invertible)) return this.host.status('Bu dönüşüm şekilleri bir çizgiye ezer: ölçek ve matris sıfır olmamalı.', 'warn');
    this.moveUnits(TRANSFORM_LABEL[spec.kind], () => ms);
  }

  restack(op: 'raise' | 'lower' | 'top' | 'bottom'): void {
    if (!this.needs(1, 'Sırasını değiştirmek için şekil seçin.')) return;
    this.run('Sıra', () => (this.host.doc.shapes = restack(this.host.doc.shapes, this.host.selection, op)));
  }

  // ── Arrays ───────────────────────────────────────────────────────────

  /** The matrices of the array tab's current settings (null when nothing is chosen). */
  arrayMatrices(): Matrix[] | null {
    const sel = this.chosen;
    const box = shapesBox(sel);
    if (!box) return null;
    const a = this.ui.array;
    const doc = this.host.doc;
    const centreOf = (at: 'box' | 'canvas' | 'point', point: Pt | null): Pt | null => (at === 'box' ? [(box.minX + box.maxX) / 2, (box.minY + box.maxY) / 2] : at === 'canvas' ? [doc.width / 2, doc.height / 2] : point);
    if (a.kind === 'rect') return rectArray(box, a.rect);
    if (a.kind === 'polar') {
      const c = centreOf(a.polar.at, a.polar.point);
      return c ? polarArray(box, { ...a.polar, centre: c }) : null;
    }
    const c = centreOf(a.mirror.at, a.mirror.point);
    return c ? [mirrorMatrix(a.mirror.axis, c, a.mirror.deg)] : null;
  }

  /** Shows (or hides) the copies the array tab would make. */
  previewArray(): void {
    const on = this.ui.tab === 'array' && this.ui.array.preview;
    this.host.canvas.setPreview(on ? this.host.selection : null, on ? this.arrayMatrices() : null);
  }

  applyArray(): void {
    const sel = this.chosen;
    const ms = this.arrayMatrices();
    if (!sel.length) return this.host.status('Çoğaltmak için şekil seçin.', 'warn');
    if (!ms) return this.host.status('Merkez noktası seçilmedi: “Tuvalde göster” ile tıklayın.', 'warn');
    if (!ms.length) return this.host.status('Kopya çıkmıyor: sayıları artırın.', 'warn');
    const copies = copiesOf(sel, ms);
    const label = this.ui.array.kind === 'mirror' ? 'Aynalı kopya' : 'Dizi';
    this.run(label, () => (this.host.doc.shapes = [...this.host.doc.shapes, ...copies]));
    this.host.status(`${label}: ${copies.length} şekil eklendi.`);
    this.previewArray();
  }

  // ── Selection helpers ────────────────────────────────────────────────

  selectAll(): void {
    this.host.select(this.host.doc.shapes.filter((s) => !s.hidden && !s.locked).map((s) => s.id));
  }

  invertSelection(): void {
    this.host.select(this.host.doc.shapes.filter((s) => !s.hidden && !s.locked && !this.host.selection.has(s.id)).map((s) => s.id));
  }

  /** Shapes painted like the chosen ones: same fill, same stroke, or both. */
  selectSame(what: 'fill' | 'stroke' | 'both' | 'kind'): void {
    const sel = this.chosen;
    if (!sel.length) return this.host.status('Benzerini seçmek için önce bir şekil seçin.', 'warn');
    const key = (s: SvgShape) => (what === 'fill' ? s.fill : what === 'stroke' ? `${s.stroke}|${s.stroke === 'none' ? '' : s.strokeWidth}` : what === 'kind' ? s.kind : `${s.fill}|${s.stroke}|${s.strokeWidth}`);
    const want = new Set(sel.map(key));
    const ids = this.host.doc.shapes.filter((s) => !s.hidden && !s.locked && want.has(key(s))).map((s) => s.id);
    this.host.select(ids);
    this.host.status(`${ids.length} şekil seçildi.`);
  }
}

const PATH_LABEL: Record<PathOpId, string> = {
  union: 'Birleşim',
  difference: 'Fark',
  intersection: 'Kesişim',
  exclusion: 'Dışlama',
  division: 'Bölme',
  cut: 'Yolu kes',
  combine: 'Tek yolda topla',
  breakApart: 'Parçalara ayır',
  split: 'Parçalara ayır (delikler kalır)',
  toPath: 'Nesneyi yola çevir',
  strokeToPath: 'Çizgiyi yola çevir',
  inset: 'İçe küçült',
  outset: 'Dışa büyüt',
  simplify: 'Sadeleştir',
  reverse: 'Yönü çevir',
  close: 'Yolu kapat',
  open: 'Yolu aç',
};

const NODE_TYPE: Record<NodeType, string> = { cusp: 'köşe', smooth: 'yumuşak', symmetric: 'simetrik', auto: 'otomatik' };
const TRANSFORM_LABEL: Record<TransformSpec['kind'], string> = { move: 'Taşı', scale: 'Ölçekle', rotate: 'Döndür', skew: 'Eğ', matrix: 'Matris' };
