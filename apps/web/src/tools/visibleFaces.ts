import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { entityFaceIndex, type Area, type FaceIndex } from '../model/geom/region';

/** Kinds that bound regions (fills, labels and dimensions do not). */
export const isBoundaryKind = (e: Entity) => e.kind !== 'point' && e.kind !== 'text' && e.kind !== 'dimension' && e.kind !== 'hatch';

/**
 * Faces closed by the visible line work, for tools that fill or create
 * areas by a click inside ("İçine tıklayarak alan", hatch by lines). The
 * faces are computed once and rebuilt only when the view, the drawing,
 * layer visibility or the boundary layer changes.
 */
export class VisibleFaces {
  /** Only this layer's line work bounds the faces; null = every visible layer. */
  layer: string | null = null;
  private readonly ctx: AppContext;
  private index: FaceIndex | null = null;
  private key = '';
  private subs: Disposable[] = [];

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  attach(): void {
    const drop = () => this.drop();
    this.subs = [this.ctx.doc.events.on('changed', drop), this.ctx.doc.layers.events.on('state', drop), this.ctx.doc.layers.events.on('structure', drop)];
  }

  detach(): void {
    this.subs.forEach((d) => d());
    this.subs = [];
    this.drop();
  }

  setLayer(layer: string | null): void {
    this.layer = layer;
    this.drop();
  }

  /** The core keeps the faces between calls: release them as soon as they are stale. */
  private drop(): void {
    this.index?.free();
    this.index = null;
  }

  at(p: Vec2, islands: boolean): Area | null {
    return this.get().at(p, islands);
  }

  private get(): FaceIndex {
    const b = this.ctx.view.camera.visibleBounds();
    const key = `${b.minX}|${b.minY}|${b.maxX}|${b.maxY}|${this.layer}`;
    if (!this.index || key !== this.key) {
      const lines = this.ctx.view.entitiesIn(b).filter((e) => (!this.layer || e.layerId === this.layer) && isBoundaryKind(e));
      this.drop();
      this.index = entityFaceIndex(lines);
      this.key = key;
    }
    return this.index;
  }
}
