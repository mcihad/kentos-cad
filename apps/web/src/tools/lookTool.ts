import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { Tool, ToolPointer } from './Tool';

/**
 * A look at the drawing for a window that stepped aside (Kenar eşleme's
 * Göster, docs/adr/0159 §9): the view may be moved, nothing is picked; a
 * click, Enter or Esc ends it and `done` brings the window back. The
 * desktop's is `kentos_interaction::look::Look`.
 */
export class LookTool implements Tool {
  readonly id = 'look';
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private readonly label: string;
  private readonly done: () => void;
  private finished = false;

  constructor(ctx: AppContext, label: string, done: () => void) {
    this.ctx = ctx;
    this.label = label;
    this.done = done;
  }

  activate(): void {
    this.prompt.set(`${this.label} [Pencereye dön (tıklama, Enter ya da Esc)]`);
  }

  deactivate(): void {
    if (!this.finished) {
      this.finished = true;
      this.done();
    }
  }

  pointerDown(p: ToolPointer): void {
    if (p.button === 0) this.finish();
  }

  confirm(): void {
    this.finish();
  }

  private finish(): void {
    if (this.finished) return;
    this.finished = true;
    this.done();
    // Leaving from inside the event would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }
}
