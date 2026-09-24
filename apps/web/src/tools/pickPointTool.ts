import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import type { Tool, ToolPointer } from './Tool';
import { pointFromText } from './tracking';

/**
 * Asks for one point on the drawing for someone else (a processing
 * parameter, a dialog): snaps and typed coordinates work as usual. The
 * callback gets the point, or null when the user leaves with Esc.
 */
export class PickPointTool implements Tool {
  readonly id = 'pickPoint';
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  readonly snaps = true;
  private readonly ctx: AppContext;
  private readonly label: string;
  private readonly done: (p: Vec2 | null) => void;
  private finished = false;
  private hover: Vec2 | null = null;

  constructor(ctx: AppContext, label: string, done: (p: Vec2 | null) => void) {
    this.ctx = ctx;
    this.label = label;
    this.done = done;
  }

  activate(): void {
    this.prompt.set(`${this.label}: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]`);
  }

  deactivate(): void {
    if (!this.finished) this.done(null);
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button === 0) this.finish(p.world);
  }

  acceptPoint(p: Vec2): boolean {
    this.finish(p);
    return true;
  }

  input(text: string): boolean {
    const p = pointFromText(this.ctx, text, null, this.hover);
    if (!p) return false;
    this.finish(p);
    return true;
  }

  confirm(): void {
    this.ctx.tools.exit();
  }

  private finish(p: Vec2): void {
    this.finished = true;
    this.done(p);
    // Leaving from inside the event would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }
}
