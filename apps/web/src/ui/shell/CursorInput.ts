import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { takesTypedInput } from '../../tools/Tool';
import { echo } from '../bottom/logPlan';
import { Component } from '../Component';
import { h } from '../dom';
import { besidePointer, type Size } from '../widgets/placeBeside';
import { namedPoint } from '../../tools/namedPoint';

/**
 * Dynamic input: while a command runs and the mouse is over the drawing,
 * typing a number or coordinate opens this field beside the cursor instead
 * of the command line, so the eyes stay on the drawing. It sends the same
 * text a command line would (tool.input); Enter applies, Esc closes.
 */
export class CursorInput extends Component {
  readonly el: HTMLElement;
  private readonly input: HTMLInputElement;
  /** What can be typed, in the project's axes (docs/adr/0165 §4). */
  private readonly hint: HTMLElement;
  private readonly ctx: AppContext;
  private open = false;
  /** The field's size, measured when it opens. */
  private size: Size = { w: 0, h: 0 };

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.input = h('input', { class: 'cursor-input__field', type: 'text', spellcheck: 'false', autocomplete: 'off', 'aria-label': 'Değer ya da koordinat' });
    this.hint = h('div', { class: 'cursor-input__hint' }, ctx.format.inputHint);
    this.el = h('div', { class: 'cursor-input', hidden: true }, this.input, this.hint);
    host.append(this.el);
    this.d.add(
      listen<KeyboardEvent>(this.input, 'keydown', (e) => {
        e.stopPropagation();
        // Space is a second Enter, as in AutoCAD (docs/adr/0018).
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.submit();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          this.close();
        } else if (e.key === 'Tab') {
          // One field for now: Tab must not move the focus away and lose the typed value (docs/adr/0018).
          e.preventDefault();
        }
      }),
    );
    this.d.add(listen(this.input, 'blur', () => this.close(false)));
    this.d.add(ctx.view.cursorWorld.subscribe(() => this.open && this.place()));
    this.d.add(ctx.tools.activeId.subscribe(() => this.close(false)));
  }

  /** Whether typed input should come here rather than to the command line. */
  accepts(): boolean {
    const { tools } = this.ctx;
    return this.ctx.prefs.cursorInput.value && this.ctx.view.cursorWorld.value !== null && takesTypedInput(tools.activeId.value, tools.active);
  }

  /** Opens the field holding the character that opened it. */
  show(first: string): void {
    this.open = true;
    this.hint.textContent = this.ctx.format.inputHint;
    this.input.value = first;
    this.el.hidden = false;
    this.size = { w: this.el.offsetWidth, h: this.el.offsetHeight };
    this.place();
    this.input.focus({ preventScroll: true });
    this.input.setSelectionRange(first.length, first.length);
  }

  private place(): void {
    const w = this.ctx.view.cursorWorld.value;
    if (!w) return;
    const camera = this.ctx.view.camera;
    // Above-right of the cursor: the tool's own measurement tag sits below-right. Near the right edge it goes
    // left of the cursor, near the top it slides down; it never leaves the drawing (placeBeside.ts).
    const at = besidePointer(camera.worldToScreen(w), this.size, { w: camera.width, h: camera.height }, { x: 18, y: -58 });
    this.el.style.transform = `translate(${at.x}px, ${at.y}px)`;
  }

  private submit(): void {
    const text = this.input.value.trim();
    const { ctx } = this;
    this.close();
    if (!text) return void ctx.commands.execute('tool.confirm');
    ctx.log.command(echo(text));
    // #ad: a point's place by its name (docs/adr/0152 §4).
    if (namedPoint(ctx, text)) return;
    const f = ctx.format;
    if (!ctx.tools.active.input?.(text)) ctx.log.warn(`“${text}” anlaşılamadı. Mesafe, ${f.pairLabel}, ${f.relativeLabel} ya da ${f.polarLabel} yazın.`);
  }

  private close(refocus = true): void {
    if (!this.open) return;
    this.open = false;
    this.el.hidden = true;
    if (refocus) this.ctx.view.focus();
  }
}
