import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { strokeGeometry } from './preview';
import type { Tool, ToolPointer } from './Tool';

/** The prompt, which a trace reads. */
export const IDENTIFY_PROMPT = 'Bilgi: bilgisi gösterilecek nesneye tıklayın';

/**
 * Bilgi (docs/adr/0225 §5; the desktop's `kentos_interaction::identify`): a click on an object opens its card beside the
 * click, in the drawing (`ToolManager.infoCard`, drawn by the shell's InfoCard): its layer's card's title, attributes,
 * text, the object's attachments and the layer's actions. Another object's click moves the card to it; a click on
 * nothing and Esc close it; Esc again leaves. The object is the click's first candidate (docs/adr/0187 §1).
 */
export class IdentifyTool implements Tool {
  readonly id = 'identify';
  readonly prompt = new Signal(IDENTIFY_PROMPT);
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private hover: Entity | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    this.ctx.tools.infoCard.set(null);
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
    this.ctx.tools.infoCard.set(null);
  }

  pointerMove(p: ToolPointer): void {
    this.hover = this.ctx.view.pick(p.screen);
    this.ctx.selection.hover.set(this.hover?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const hit = this.ctx.view.pick(p.screen);
    const at: Vec2 = p.raw;
    this.ctx.tools.infoCard.set(hit ? { id: hit.id, at: { x: at.x, y: at.y } } : null);
    this.ctx.view.requestOverlay();
  }

  confirm(): void {
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    // Esc closes the card first; then it leaves.
    if (!this.ctx.tools.infoCard.value) return false;
    this.ctx.tools.infoCard.set(null);
    this.ctx.view.requestOverlay();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const shown = this.ctx.tools.infoCard.value;
    const e = shown ? this.ctx.doc.get(shown.id) : undefined;
    if (e) strokeGeometry(g, view, entityGeometry(e), { color: pal.accent, width: 2.5 });
    if (this.hover && this.hover.id !== e?.id) strokeGeometry(g, view, entityGeometry(this.hover), { color: pal.snap, dash: [5, 3], width: 2 });
  }
}
