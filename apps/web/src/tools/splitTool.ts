import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { entityGeometry, type Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { nearestS, pathOf } from '../model/ops/path';
import { splitAtCrossings, splitByLength, splitEqual, type SplitPieces } from '../model/ops/split';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { createdIds, editGeometry, uidOf, writeEdit } from './editCommand';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { drawTag, strokeGeometry } from './preview';
import { strokePieces } from './reshapePreview';
import type { ToolPointer } from './Tool';

type Mode = 'crossings' | 'equal' | 'length';

/** What Parçala cuts: lines, open polylines, arcs and circles. */
const splittable = (e: Entity) => e.kind === 'line' || e.kind === 'polyline' || e.kind === 'arc' || e.kind === 'circle';

/**
 * Parçala (docs/adr/0140): cuts line work into separate objects.
 *
 * - Kesişimlerden (default): the selected objects come apart where they
 *   cross each other. Objects are picked before or after; the pieces are
 *   previewed and Enter (or right click) cuts.
 * - Eşit parçalara (E): click an object, type the number of parts.
 * - Uzunluktan (U): click an object near the end to measure from, type the
 *   length of the pieces.
 *
 * The first piece of an object keeps its place and persistent id, the others
 * are new objects with its layer, colour, attributes and label; all of a
 * confirm is one edit (`cad.entities.edit`, operation `split`).
 */
export class SplitTool extends SelectionFirstTool {
  readonly id = 'split';
  protected readonly label = 'Parçala';
  private static partCount = 4;
  private static pieceLength = 10;
  private mode: Mode = 'crossings';
  /** Equal and length modes: the object clicked and the end measuring starts from. */
  private target: { entity: Entity; fromEnd: boolean; length: number } | null = null;
  /** The pieces previewed: each object that comes apart and its pieces. */
  private cuts: { entity: Entity; pieces: Entity[] }[] = [];
  private mouse: Vec2 | null = null;
  /** Selected objects Kesişimlerden left out: not cuttable, on a locked layer. */
  private left = { kind: 0, locked: 0 };

  private get picks(): boolean {
    return this.mode !== 'crossings';
  }

  // ── Kesişimlerden ────────────────────────────────────────────────────

  protected begin(): void {
    if (this.picks) return;
    const { doc } = this.ctx;
    const all = this.targets();
    const cuttable = all.filter(splittable);
    const editable = cuttable.filter((e) => !doc.layers.isLocked(e.layerId));
    // Said when the cut is confirmed, not while the objects are still being chosen.
    this.left = { kind: all.length - cuttable.length, locked: cuttable.length - editable.length };
    this.cuts = splitAtCrossings(editable).map((s: SplitPieces) => ({ entity: editable[s.index], pieces: s.pieces }));
  }

  protected stagePrompt(): string {
    if (!this.cuts.length) return `seçilen nesneler birbirini kesmiyor; Esc ile seçime dönün [${this.otherModes()}]`;
    const pieces = this.cuts.reduce((n, c) => n + c.pieces.length, 0);
    return `${this.cuts.length} nesne ${pieces} parçaya bölünecek [Böl (Enter) / ${this.otherModes()}]`;
  }

  protected point(): void {
    this.applyCrossings();
  }

  private applyCrossings(): void {
    const { log } = this.ctx;
    const said = [
      ...(this.left.kind ? [`${this.left.kind} nesne çizgi, açık çoklu çizgi, yay ya da daire olmadığı için atlandı.`] : []),
      ...(this.left.locked ? [`${this.left.locked} nesne kilitli katmanda olduğu için atlandı.`] : []),
    ];
    if (!this.cuts.length) return void log.warn(['Seçilen nesneler birbirini kesmiyor; bölünecek yer yok. Esc ile kesişen nesneleri seçin.', ...said].join(' '));
    for (const w of said) log.warn(w);
    const pieces = this.cuts.reduce((n, c) => n + c.pieces.length, 0);
    if (!this.write(this.cuts)) return;
    log.success(`${this.cuts.length} nesne ${pieces} parçaya bölündü.`);
    // Done: the manager asks `cancel` before it leaves, and a tool already back at picking lets it.
    this.picking = true;
    this.cuts = [];
    this.ctx.tools.exit();
  }

  /** Writes the cuts as one edit and selects everything that came of them. */
  private write(cuts: { entity: Entity; pieces: Entity[] }[]): boolean {
    const changes: EntityEdit[] = cuts.flatMap(({ entity, pieces }) => {
      const uid = uidOf(this.ctx, entity);
      const [first, ...rest] = pieces;
      return [
        { kind: 'replace', uid, geometry: editGeometry(entityGeometry(first)), keepData: true } as EntityEdit,
        ...rest.map((p): EntityEdit => ({ kind: 'add', from: uid, geometry: editGeometry(entityGeometry(p)), keepData: true })),
      ];
    });
    const out = writeEdit(this.ctx, 'split', changes);
    if (!out) return false;
    // Everything that came of the cut is selected, as after Birleştir and Patlat.
    const changed = out.changed.map((uid) => this.ctx.doc.byUid(uid)?.id).filter((id): id is number => id !== undefined);
    this.ctx.selection.set([...changed, ...createdIds(this.ctx, out)]);
    return true;
  }

  // ── Eşit parçalara, Uzunluktan ───────────────────────────────────────

  private tryPieces(t: NonNullable<SplitTool['target']>): Entity[] | null {
    return this.mode === 'equal' ? splitEqual(t.entity, SplitTool.partCount) : splitByLength(t.entity, SplitTool.pieceLength, t.fromEnd);
  }

  private preview(): void {
    const t = this.target;
    const pieces = t ? this.tryPieces(t) : null;
    this.cuts = t && pieces ? [{ entity: t.entity, pieces }] : [];
  }

  private commitTarget(): void {
    const t = this.target;
    if (!t) return;
    const { log, format } = this.ctx;
    if (this.mode === 'length' && SplitTool.pieceLength >= t.length - 1e-9) return void log.warn(`Uzunluk nesne boyundan küçük olmalı; nesne ${format.length(t.length)} uzunluğunda, yazılan ${format.length(SplitTool.pieceLength)}.`);
    this.preview();
    if (!this.cuts.length) return void log.warn('Bu değerle nesne bölünemez; parça sayısını ya da uzunluğu değiştirin.');
    const n = this.cuts[0].pieces.length;
    if (!this.write(this.cuts)) return;
    log.success(this.mode === 'equal' ? `Nesne ${n} eşit parçaya bölündü.` : `Nesne ${n} parçaya bölündü: ${format.length(SplitTool.pieceLength)} uzunlukta, ${t.fromEnd ? 'sondan' : 'baştan'} ölçülü.`);
    this.target = null;
    this.cuts = [];
    this.refresh();
  }

  private pickTarget(p: ToolPointer): void {
    const { view, doc, log, selection } = this.ctx;
    const e = view.pickEdge(p.screen, (x) => splittable(x) && !doc.layers.isLocked(x.layerId));
    if (!e) return log.warn('Bölünecek düzenlenebilir bir çizgiye, açık çoklu çizgiye, yaya ya da daireye tıklayın.');
    const path = pathOf(e);
    if (!path || path.length < 1e-9) return log.warn('Bu nesne bölünemez.');
    this.target = { entity: e, fromEnd: !path.closed && nearestS(path, p.raw) > path.length / 2, length: path.length };
    selection.hover.set(null);
    this.preview();
    this.refresh();
  }

  // ── The pointer ──────────────────────────────────────────────────────

  override pointerDown(p: ToolPointer): void {
    if (!this.picks) return super.pointerDown(p);
    if (p.button !== 0) return;
    if (this.target) return this.commitTarget();
    this.pickTarget(p);
  }

  override pointerMove(p: ToolPointer): void {
    this.mouse = p.raw;
    if (!this.picks) return super.pointerMove(p);
    if (!this.target) this.ctx.selection.hover.set(this.ctx.view.pickEdge(p.screen, (x) => splittable(x) && !this.ctx.doc.layers.isLocked(x.layerId))?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  override pointerUp(p: ToolPointer): void {
    if (!this.picks) super.pointerUp(p);
  }

  override acceptPoint(): boolean {
    return false;
  }

  // ── Typed input, options ─────────────────────────────────────────────

  private setMode(mode: Mode): void {
    if (this.mode === mode) return;
    this.mode = mode;
    this.target = null;
    this.cuts = [];
    this.ctx.selection.hover.set(null);
    if (mode === 'crossings') {
      this.picking = this.ctx.selection.size === 0;
      if (!this.picking) this.begin();
    }
    this.refresh();
  }

  override input(text: string): boolean {
    const t = text.trim().toLocaleUpperCase('tr-TR');
    if (t === 'E' || t === 'U' || t === 'K') {
      this.setMode(t === 'E' ? 'equal' : t === 'U' ? 'length' : 'crossings');
      return true;
    }
    if (!this.picks) return false;
    const n = parseNumber(text);
    if (n === null || /[,;@<]/.test(text)) return false;
    const { log } = this.ctx;
    if (this.mode === 'equal') {
      if (!Number.isInteger(n) || n < 2 || n > 10_000) {
        log.warn('Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.');
        return true;
      }
      SplitTool.partCount = n;
    } else {
      if (!(n > 0)) {
        log.warn('Parça uzunluğu sıfırdan büyük olmalı.');
        return true;
      }
      SplitTool.pieceLength = n;
    }
    this.preview();
    this.refresh();
    return true;
  }

  override confirm(): void {
    if (this.picks) return this.target ? this.commitTarget() : this.ctx.tools.exit();
    if (this.picking) return super.confirm();
    this.applyCrossings();
  }

  cancel(): boolean {
    if (this.picks) {
      if (!this.target) return false;
      this.target = null;
      this.cuts = [];
      this.refresh();
      return true;
    }
    if (this.picking) return false;
    // One step back: from the preview to the picking of objects.
    this.picking = true;
    this.cuts = [];
    this.refresh();
    return true;
  }

  private otherModes(): string {
    const all: [Mode, string][] = [['crossings', 'Kesişimlerden (K)'], ['equal', 'Eşit parçalara (E)'], ['length', 'Uzunluktan (U)']];
    return all.filter(([m]) => m !== this.mode).map(([, label]) => label).join(' / ');
  }

  protected override pickHint(): string {
    return `[${this.otherModes()}]`;
  }

  protected override refresh(): void {
    if (!this.picks) return super.refresh();
    const f = this.ctx.format;
    const value = this.mode === 'equal' ? `${SplitTool.partCount} parça` : `parça ${f.length(SplitTool.pieceLength)}`;
    const ask = this.mode === 'equal' ? 'parça sayısını yazın' : 'parça uzunluğunu yazın';
    const step = this.target
      ? `${ask} ya da Enter ile böl [Böl (Enter); ${value}; ${this.otherModes()}]`
      : `bölünecek nesneye tıklayın${this.mode === 'length' ? ' (ölçme yakın uçtan başlar)' : ''} [${value}; ${this.otherModes()}]`;
    this.prompt.set(`Parçala: ${step}`);
    this.ctx.view.requestOverlay();
  }

  // ── Preview ──────────────────────────────────────────────────────────

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking && !this.picks) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    if (this.picks && !this.target) return;
    const pieces = this.cuts.reduce((n, c) => n + c.pieces.length, 0);
    for (const c of this.cuts.slice(0, MAX_GHOSTS)) {
      if (this.picks) strokeGeometry(g, view, entityGeometry(c.entity), { color: pal.danger, dash: [5, 3], width: 1.5 });
      strokePieces(g, view, c.pieces, [pal.snap, pal.fg], pal.labelHalo, c.entity.kind === 'circle');
    }
    if (!this.mouse || !pieces) return;
    const f = this.ctx.format;
    const lines = this.picks ? [`${pieces} parça`, ...(this.mode === 'length' && this.target ? [`${f.length(SplitTool.pieceLength)} + kalan`] : []), 'Tıklayın: böl'] : [`${this.cuts.length} nesne, ${pieces} parça`, 'Tıklayın: böl'];
    drawTag(g, view.worldToScreen(this.mouse), lines, pal.accent, pal.labelHalo);
  }
}
