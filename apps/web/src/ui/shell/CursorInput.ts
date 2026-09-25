import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { Component } from '../Component';
import { h } from '../dom';

/**
 * Dynamic input: while a command runs and the mouse is over the drawing,
 * typing a number or coordinate opens this field beside the cursor instead
 * of the command line, so the eyes stay on the drawing. It sends the same
 * text a command line would (tool.input); Enter applies, Esc closes.
 */
export class CursorInput extends Component {
  readonly el: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly ctx: AppContext;
  private open = false;

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.input = h('input', { class: 'cursor-input__field', type: 'text', spellcheck: 'false', autocomplete: 'off', 'aria-label': 'Değer ya da koordinat' });
    this.el = h(
      'div',
      { class: 'cursor-input', hidden: true },
      this.input,
      h('div', { class: 'cursor-input__hint' }, 'mesafe · Y,X · @dY,dX · @mesafe<açı'),
    );
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
    return this.ctx.prefs.cursorInput.value && this.ctx.view.cursorWorld.value !== null && this.ctx.tools.activeId.value !== 'select';
  }

  /** Opens the field holding the character that opened it. */
  show(first: string): void {
    this.open = true;
    this.input.value = first;
    this.el.hidden = false;
    this.place();
    this.input.focus({ preventScroll: true });
    this.input.setSelectionRange(first.length, first.length);
  }

  private place(): void {
    const w = this.ctx.view.cursorWorld.value;
    if (!w) return;
    const s = this.ctx.view.camera.worldToScreen(w);
    // Above-right of the cursor: the tool's own measurement tag sits below-right.
    this.el.style.transform = `translate(${Math.round(s.x + 18)}px, ${Math.round(s.y - 58)}px)`;
  }

  private submit(): void {
    const text = this.input.value.trim();
    const { ctx } = this;
    this.close();
    if (!text) return void ctx.commands.execute('tool.confirm');
    ctx.log.command(`› ${text}`);
    if (!ctx.tools.active.input?.(text)) ctx.log.warn(`“${text}” anlaşılamadı. Mesafe, Y,X, @dY,dX ya da @mesafe<açı yazın.`);
  }

  private close(refocus = true): void {
    if (!this.open) return;
    this.open = false;
    this.el.hidden = true;
    if (refocus) this.ctx.view.focus();
  }
}
