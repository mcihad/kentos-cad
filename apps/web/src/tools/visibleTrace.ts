import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import type { Vec2 } from '../model/geometry';
import { traceGraph, tracedKind, type Traced, type TraceGraph } from '../model/ops/trace';

/**
 * İzle's graph of the visible line work (docs/adr/0161 §1, §5) for the path tools: built once and rebuilt only when
 * the view or the drawing (its layers too) changes, as VisibleFaces does for the faces; the core keeps it between
 * calls, and it is released as soon as it is stale. The desktop's is `crates/native/interaction/src/trace_work.rs`.
 */
export class VisibleTrace {
  private readonly ctx: AppContext;
  private graph: TraceGraph | null = null;
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

  /** The shortest way from `a` to `b` along the visible line work, or null. */
  path(a: Vec2, b: Vec2): Traced | null {
    return this.get().path(a, b);
  }

  /** The point of the visible line work nearest to `p` within `reach` metres, or null. */
  nearest(p: Vec2, reach: number): Vec2 | null {
    return this.get().nearest(p, reach);
  }

  private drop(): void {
    this.graph?.free();
    this.graph = null;
  }

  private get(): TraceGraph {
    const b = this.ctx.view.camera.visibleBounds();
    const key = `${b.minX}|${b.minY}|${b.maxX}|${b.maxY}`;
    if (!this.graph || key !== this.key) {
      this.drop();
      this.graph = traceGraph(this.ctx.view.entitiesIn(b).filter(tracedKind));
      this.key = key;
    }
    return this.graph;
  }
}
