import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { Entity, EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { LibraryAsset } from '../model/style';
import { hasPicture } from '../product/entitiesEdit';
import type { ViewTransform } from '../viewport/Camera';
import { parseLength } from './coordinateInput';
import { imageClip, imagePlaced } from './constructions';
import * as createCommand from './createCommand';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { pictureItem, PICTURE_FILES } from './pictureFile';
import { drawTag, strokePath } from './preview';
import type { Tool, ToolCursor, ToolPointer } from './Tool';

/**
 * The picture tools (docs/adr/0192 §5), twins of the desktop's `kentos_interaction::image_insert` and
 * `::image_clip`.
 *
 * Resim ekle: as the tool starts it asks for a PNG or JPEG (the click that started it lets the browser open the
 * picker; a cancelled picker leaves); then the picture's lower left corner, then a second point: its width the
 * distance, its turn the direction (the core's `imagePlaced`), its height from the picture's own shape; a typed
 * number after the corner is the width, unturned. The picture is kept in the project's library once (an edit, no undo
 * step; docs/adr/0092): the browser never knows a file's path, so the web offers no Bağlı. One step “Resim ekle”
 * (`cad.entities.create`'s `image`) on the active layer, and the tool leaves.
 *
 * Resmi kırp: a picture's edge is clicked; its boundary is drawn as a rectangle by two corners (Dikdörtgen, D) or as
 * a polygon by its corners and Enter (Çokgen, Ç); the core cuts it to the picture in its own fractions
 * (`imageClip`). Kaldır (K) takes the clip away. One step “Resmi kırp” (`cad.entities.edit`'s `imageClip`); the tool
 * waits for the next picture. Esc and Ctrl+Z take back the last corner, then the picture; Esc with none leaves.
 */

export const INSERT_LABEL = 'Resim ekle';
export const CLIP_LABEL = 'Resmi kırp';
/** A click on no picture. */
export const NO_PICTURE = 'Tıklanan yerde kilitsiz bir resim yok; kırpılacak resmin kenarına tıklayın.';

interface PictureFile {
  name: string;
  id: string;
  item: LibraryAsset;
  width: number;
  height: number;
}

/** The frame's corners, counter-clockwise from the lower left. */
function frameCorners(p: Vec2, width: number, height: number, rotation: number): Vec2[] {
  const [c, s] = [Math.cos(rotation), Math.sin(rotation)];
  const u = { x: width * c, y: width * s };
  const v = { x: -height * s, y: height * c };
  return [p, { x: p.x + u.x, y: p.y + u.y }, { x: p.x + u.x + v.x, y: p.y + u.y + v.y }, { x: p.x + v.x, y: p.y + v.y }];
}

/** The part of a picture shown, in the world: its clip's corners, or its frame's. */
function shown(e: Extract<Entity, { kind: 'image' }>): Vec2[] {
  const corners = frameCorners(e.p, e.width, e.height, e.rotation);
  if (!e.clip || e.clip.length < 3) return corners;
  const [p, a, , b] = corners;
  const u = { x: a.x - p.x, y: a.y - p.y };
  const v = { x: b.x - p.x, y: b.y - p.y };
  return e.clip.map((q) => {
    const t = e.mirror ? 1 - q.y : q.y;
    return { x: p.x + u.x * q.x + v.x * t, y: p.y + u.y * q.x + v.y * t };
  });
}

export class ImageInsertTool implements Tool {
  readonly id = 'imageInsert';
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  private readonly ctx: AppContext;
  private file: PictureFile | null = null;
  private corner: Vec2 | null = null;
  private hover: Vec2 | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  get snaps(): boolean {
    return this.file !== null;
  }

  get pointCount(): number {
    return this.corner ? 1 : 0;
  }

  activate(): void {
    this.refresh();
    void this.pick();
  }

  private async pick(): Promise<void> {
    const picked = await this.ctx.files.pickForImport(PICTURE_FILES);
    if (!picked) return this.ctx.tools.exit();
    const got = await pictureItem(picked.name, picked.bytes);
    if (!got.ok) {
      this.ctx.log.warn(got.error);
      return this.ctx.tools.exit();
    }
    this.file = { name: picked.name, id: got.id, item: got.item, width: got.item.width, height: got.item.height };
    this.refresh();
  }

  private refresh(): void {
    let step: string;
    if (!this.file) step = 'resim dosyasını seçin (PNG ya da JPEG)';
    else if (!this.corner) step = `“${this.file.name}” için sol alt köşeye tıklayın`;
    else step = 'genişliği yazın ya da ikinci noktaya tıklayın (yön dönüştür)';
    this.prompt.set(`${INSERT_LABEL}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  /** The picture's height over its width. */
  private get aspect(): number {
    return this.file ? this.file.height / Math.max(this.file.width, 1) : 1;
  }

  /** Places the picture from its corner to `q` and leaves; why not otherwise. */
  private write(q: Vec2): void {
    const { corner, file } = this;
    if (!corner || !file) return;
    const got = imagePlaced(corner, q, this.aspect);
    if (!got.placed) return void this.ctx.log.warn(got.problem ?? '');
    const at = got.placed;
    const doc = this.ctx.doc;
    // The picture in the project's library, once (docs/adr/0192 §2).
    if (!hasPicture(doc, file.id)) doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, file.item] });
    const geometry = { kind: 'image', p: corner, width: at.width, height: at.height, rotation: at.rotation, asset: file.id } as EntityGeometry;
    if (!createCommand.writeObjects(this.ctx, [geometry], 'image')) return;
    const f = this.ctx.format;
    this.ctx.log.success(`${INSERT_LABEL}: ${file.name} yerleştirildi (${f.length(at.width)} × ${f.length(at.height)}).`);
    this.leave();
  }

  /** Leaves the tool (the manager's exit asks `cancel` first, which would only take the corner back). */
  private leave(): void {
    this.corner = null;
    this.ctx.tools.exit();
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0 || !this.file) return;
    if (!this.corner) {
      this.corner = p.world;
      this.refresh();
    } else this.write(p.world);
  }

  input(text: string): boolean {
    const corner = this.corner;
    if (!this.file || !corner) return false;
    const w = parseLength(this.ctx.format, text.trim());
    if (w === null || !(w > 0) || !Number.isFinite(w)) return false;
    this.write({ x: corner.x + w, y: corner.y });
    return true;
  }

  confirm(): void {
    this.leave();
  }

  /** Esc: the corner goes first. */
  cancel(): boolean {
    return this.undoStep();
  }

  undoStep(): boolean {
    if (!this.corner) return false;
    this.corner = null;
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const { corner, hover } = this;
    if (!corner || !hover) return;
    const got = imagePlaced(corner, hover, this.aspect);
    if (!got.placed) return;
    const at = got.placed;
    const [a, b, c, d] = frameCorners(corner, at.width, at.height, at.rotation);
    const pal = this.ctx.view.palette;
    strokePath(g, view, [a, b, c, d, a], { color: pal.accent, dash: [6, 4] });
    // Its diagonals, as a picture's place is drawn before its pixels.
    strokePath(g, view, [a, c], { color: pal.snap });
    strokePath(g, view, [b, d], { color: pal.snap });
    const f = this.ctx.format;
    drawTag(g, view.worldToScreen(hover), [`${f.length(at.width)} × ${f.length(at.height)}`], pal.accent, pal.labelHalo);
  }
}

export class ImageClipTool implements Tool {
  readonly id = 'imageClip';
  readonly prompt = new Signal('');
  /** Dikdörtgen or Çokgen, remembered across runs (the desktop's `Memory::image_clip_polygon`). */
  private static polygon = false;
  private readonly ctx: AppContext;
  private picture: Extract<Entity, { kind: 'image' }> | null = null;
  private pts: Vec2[] = [];
  private hover: Vec2 | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  get snaps(): boolean {
    return this.picture !== null;
  }

  get cursor(): ToolCursor {
    return this.picture ? 'cross' : 'pick';
  }

  get pointCount(): number {
    return (this.picture ? 1 : 0) + this.pts.length;
  }

  activate(): void {
    this.refresh();
  }

  private refresh(): void {
    const picture = this.picture;
    if (!picture) {
      this.prompt.set(`${CLIP_LABEL}: kırpılacak resmin kenarına tıklayın`);
    } else {
      const polygon = ImageClipTool.polygon;
      const n = this.pts.length;
      const step = !polygon ? (n === 0 ? 'sınırın ilk köşesine tıklayın' : 'sınırın karşı köşesine tıklayın') : n < 3 ? 'sınırın köşelerine tıklayın' : 'sonraki köşeye tıklayın ya da Enter ile bitirin';
      const options = [polygon ? 'Dikdörtgen (D)' : 'Dikdörtgen (D): açık', polygon ? 'Çokgen (Ç): açık' : 'Çokgen (Ç)', ...(picture.clip ? ['Kaldır (K)'] : [])];
      this.prompt.set(`${CLIP_LABEL}: ${step} [${options.join(' / ')}]`);
    }
    this.ctx.view.requestOverlay();
  }

  private editable(e: Entity): boolean {
    return e.kind === 'image' && !this.ctx.doc.layers.isLocked(e.layerId);
  }

  private pickAt(p: ToolPointer): Entity | null {
    return this.ctx.view.pickEdge(p.screen, (e) => this.editable(e));
  }

  /** Writes the picture with `clip` (null: the whole picture) and waits for the next one. */
  private write(clip: Vec2[] | null): void {
    const picture = this.picture;
    if (!picture) return;
    const now = this.ctx.doc.get(picture.id);
    if (!now || now.kind !== 'image') return;
    const geometry = { ...now } as Record<string, unknown>;
    if (clip) geometry.clip = clip;
    else delete geometry.clip;
    if (!writeEdit(this.ctx, 'imageClip', [{ kind: 'update', uid: uidOf(this.ctx, now.id), geometry: editGeometry(geometry as unknown as EntityGeometry) }])) return;
    this.ctx.log.success(clip ? `${CLIP_LABEL}: resim kırpıldı.` : `${CLIP_LABEL}: kırpma kaldırıldı.`);
    this.picture = null;
    this.pts = [];
  }

  /** The boundary drawn so far as a clip, written; why not otherwise. */
  private clip(ring: Vec2[]): void {
    const picture = this.picture;
    if (!picture) return;
    const got = imageClip(picture, ring);
    if (got.clip) this.write(got.clip);
    else {
      this.ctx.log.warn(got.problem ?? '');
      this.pts = [];
    }
  }

  /** The rectangle of two corners. */
  private static rectangle(a: Vec2, b: Vec2): Vec2[] {
    return [a, { x: b.x, y: a.y }, b, { x: a.x, y: b.y }];
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    if (!this.picture) this.ctx.selection.hover.set(this.pickAt(p)?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (!this.picture) {
      const e = this.pickAt(p);
      if (e?.kind === 'image') {
        this.picture = e;
        this.ctx.selection.hover.set(null);
      } else this.ctx.log.warn(NO_PICTURE);
      return this.refresh();
    }
    this.pts.push(p.world);
    if (!ImageClipTool.polygon && this.pts.length === 2) this.clip(ImageClipTool.rectangle(this.pts[0], this.pts[1]));
    this.refresh();
  }

  input(text: string): boolean {
    if (!this.picture) return false;
    switch (text.trim().toLocaleUpperCase('tr-TR')) {
      case 'D':
        ImageClipTool.polygon = false;
        this.pts = [];
        break;
      case 'Ç':
      case 'C':
        ImageClipTool.polygon = true;
        this.pts = [];
        break;
      case 'K':
        if (!this.picture.clip) return false;
        this.write(null);
        break;
      default:
        return false;
    }
    this.refresh();
    return true;
  }

  /** Enter: a polygon of three corners or more is the boundary; else the tool leaves. */
  confirm(): void {
    if (ImageClipTool.polygon && this.pts.length >= 3) {
      const ring = this.pts;
      this.pts = [];
      this.clip(ring);
      return this.refresh();
    }
    // Leaves, the picture and its corners with it (the manager's exit asks `cancel` first).
    this.picture = null;
    this.pts = [];
    this.ctx.tools.exit();
  }

  /** Esc: the last corner, then the picture go first. */
  cancel(): boolean {
    return this.undoStep();
  }

  undoStep(): boolean {
    if (this.pts.length) this.pts.pop();
    else if (this.picture) this.picture = null;
    else return false;
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const picture = this.picture;
    if (!picture) return;
    const pal = this.ctx.view.palette;
    // The picture's part shown now, then the boundary drawn so far.
    const now = shown(picture);
    strokePath(g, view, [...now, now[0]], { color: pal.snap, width: 2 });
    let ring = [...this.pts];
    const h = this.hover;
    if (h) {
      if (!ImageClipTool.polygon && ring.length === 1) ring = ImageClipTool.rectangle(ring[0], h);
      else if (ring.length) ring.push(h);
    }
    if (ring.length >= 2) strokePath(g, view, [...ring, ring[0]], { color: pal.accent, dash: [6, 4] });
  }
}
