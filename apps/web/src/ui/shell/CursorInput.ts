import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { takesTypedInput } from '../../tools/Tool';
import { parseNumber } from '../../tools/coordinateInput';
import { echo } from '../bottom/logPlan';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { besidePointer, type Size } from '../widgets/placeBeside';
import { namedPoint } from '../../tools/namedPoint';
import {
  hasLocks,
  lockedDirection,
  lockLength,
  lockPoint,
  lockReference,
  lockToward,
  lockTravel,
  lockWords,
  parseLockText,
  typedLock,
  type LockAsk,
} from '../../tools/locks';

/** What the typed text is: a point (as the command line takes it), or the value of a lock (docs/adr/0166 §6). */
type Mode = 'point' | LockAsk;

/** A plain typed number (the point grammar's distance, a decimal comma allowed): what Tab locks as a length. */
const PLAIN_NUMBER = /^[-+]?\d+(?:[.,]\d+)?$/;

/**
 * Dynamic input: while a command runs and the mouse is over the drawing,
 * typing a number or coordinate opens this field beside the cursor instead
 * of the command line, so the eyes stay on the drawing. It sends the same
 * text a command line would (tool.input); Enter applies, Esc closes.
 *
 * The digitizing locks (docs/adr/0166 §6): a number and Tab lock the next
 * point's length and turn the field to its direction (AutoCAD's dynamic
 * input); `<45` locks the direction. Uzunluk…, Açı… and Sapma… from the
 * menu open it under their name. Enter with a number in a lock's field
 * locks it, and places the point once its length and direction are both
 * held. The locks show as chips under the field, each with its ×.
 */
export class CursorInput extends Component {
  readonly el: HTMLElement;
  private readonly input: HTMLInputElement;
  /** The lock the field's number is for (Uzunluk, Açı or Semt, Sapma); hidden for a point. */
  private readonly label: HTMLElement;
  /** The locks now held, as chips. */
  private readonly locks: HTMLElement;
  /** What can be typed, in the project's axes (docs/adr/0165 §4). */
  private readonly hint: HTMLElement;
  private readonly ctx: AppContext;
  private open = false;
  private mode: Mode = 'point';
  /** The field's size, measured when it opens. */
  private size: Size = { w: 0, h: 0 };

  constructor(ctx: AppContext, host: HTMLElement) {
    super();
    this.ctx = ctx;
    this.input = h('input', { class: 'cursor-input__field', type: 'text', spellcheck: 'false', autocomplete: 'off', 'aria-label': 'Değer ya da koordinat' });
    this.label = h('span', { class: 'cursor-input__label', hidden: true });
    this.locks = h('div', { class: 'cursor-input__locks', hidden: true });
    this.hint = h('div', { class: 'cursor-input__hint' }, ctx.format.inputHint);
    this.el = h('div', { class: 'cursor-input', hidden: true }, h('div', { class: 'cursor-input__row' }, this.label, this.input), this.locks, this.hint);
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
          // Tab never moves the focus away and loses the typed value (docs/adr/0018): it locks it (docs/adr/0166 §6).
          e.preventDefault();
          this.tab();
        }
      }),
    );
    this.d.add(listen(this.input, 'blur', () => this.close(false)));
    this.d.add(ctx.view.cursorWorld.subscribe(() => this.open && this.place()));
    this.d.add(ctx.tools.activeId.subscribe(() => this.close(false)));
    this.d.add(ctx.settings.locks.subscribe(() => this.open && this.renderLocks()));
    // A lock chosen from the menu asks for its value here.
    this.d.add(
      ctx.settings.lockAsk.subscribe((kind) => {
        if (!kind) return;
        ctx.settings.lockAsk.set(null);
        this.ask(kind);
      }),
    );
  }

  /** Whether typed input should come here rather than to the command line. */
  accepts(): boolean {
    const { tools } = this.ctx;
    return this.ctx.prefs.cursorInput.value && this.ctx.view.cursorWorld.value !== null && takesTypedInput(tools.activeId.value, tools.active);
  }

  /** Opens the field holding the character that opened it. */
  show(first: string): void {
    this.openAs('point', first);
  }

  /** Opens the field for a lock's value, empty, under the lock's name. */
  ask(kind: LockAsk): void {
    this.openAs(kind, '');
  }

  private openAs(mode: Mode, text: string): void {
    this.open = true;
    this.ctx.settings.valueCard.set(true);
    this.ctx.view.requestOverlay();
    this.setMode(mode);
    this.input.value = text;
    this.renderLocks();
    this.el.hidden = false;
    this.size = { w: this.el.offsetWidth, h: this.el.offsetHeight };
    this.place();
    this.input.focus({ preventScroll: true });
    this.input.setSelectionRange(text.length, text.length);
  }

  private setMode(mode: Mode): void {
    this.mode = mode;
    const f = this.ctx.format;
    const name = mode === 'length' ? 'Uzunluk' : mode === 'angle' ? f.directionName : mode === 'deflection' ? 'Sapma' : '';
    this.label.textContent = name;
    this.label.hidden = mode === 'point';
    this.input.setAttribute('aria-label', mode === 'point' ? 'Değer ya da koordinat' : `${name} kilidi`);
    this.hint.textContent = mode === 'point' ? f.inputHint : 'Enter kilitler · Tab öbür değere geçer · Esc kapatır';
  }

  /** The locks as chips: the lock icon, the words and a × that lets that lock go. */
  private renderLocks(): void {
    const s = this.ctx.settings.locks.value;
    this.locks.hidden = !hasLocks(s);
    const chip = (text: string, drop: () => void) => {
      const x = h('button', { class: 'cursor-input__drop', type: 'button', tabindex: '-1', 'aria-label': `${text} kilidini kaldır` }, icon('close', 11));
      // The field keeps the keyboard.
      x.addEventListener('pointerdown', (e) => {
        e.preventDefault();
        drop();
        this.ctx.view.requestOverlay();
      });
      return h('span', { class: 'cursor-input__lock' }, icon('lock', 11), h('span', { class: 'cursor-input__lock-text' }, text), x);
    };
    const words = lockWords(this.ctx, { ...s, keep: false });
    const chips = words.map((w, i) =>
      chip(w, () => {
        const now = this.ctx.settings.locks.value;
        // The length's chip comes first when there is one.
        this.ctx.settings.locks.set(now.length !== null && i === 0 ? { ...now, length: null } : { ...now, toward: null, edge: null });
      }),
    );
    replaceChildren(this.locks, ...chips);
    if (this.open) this.size = { w: this.el.offsetWidth, h: this.el.offsetHeight };
  }

  private place(): void {
    const w = this.ctx.view.cursorWorld.value ?? lockReference(this.ctx);
    if (!w) return;
    const camera = this.ctx.view.camera;
    // Above-right of the cursor, its foot 8 px above it however many locks it lists: the tool's own measurement tag
    // sits below-right. Near the right edge it goes left of the cursor, near the top it slides down; it never leaves
    // the drawing (placeBeside.ts; the desktop's `above_right`).
    const at = besidePointer(camera.worldToScreen(w), this.size, { w: camera.width, h: camera.height }, { x: 18, y: -(this.size.h + 8) });
    this.el.style.transform = `translate(${at.x}px, ${at.y}px)`;
  }

  /** Tab: the typed number locks its value and the field turns to the other one (length ↔ direction). */
  private tab(): void {
    const text = this.input.value.trim();
    const { ctx } = this;
    if (this.mode === 'point') {
      const angle = parseLockText(text);
      if (angle !== null) {
        if (lockToward(ctx, { kind: 'angle', value: angle })) this.next('length');
        return;
      }
      const n = PLAIN_NUMBER.test(text) ? parseNumber(text) : null;
      if (n !== null && lockLength(ctx, ctx.format.toMetres(n))) this.next('angle');
      return;
    }
    if (this.lockTyped(text)) this.next(this.mode === 'length' ? 'angle' : 'length');
  }

  /** The field turns to another lock's value, empty. */
  private next(mode: Mode): void {
    this.setMode(mode);
    this.input.value = '';
    this.renderLocks();
    this.place();
  }

  /** The number typed in a lock's field locks that; false (and why, when it is not a number) when nothing was locked. */
  private lockTyped(text: string): boolean {
    const { ctx } = this;
    const n = parseNumber(text);
    if (n === null) {
      if (text) ctx.log.warn(`“${text}” bir sayı değil; ${this.label.textContent?.toLocaleLowerCase('tr-TR')} için sayı yazın.`);
      return false;
    }
    if (this.mode === 'length') return lockLength(ctx, ctx.format.toMetres(n));
    return lockToward(ctx, { kind: this.mode === 'deflection' ? 'deflection' : 'angle', value: n });
  }

  private submit(): void {
    const text = this.input.value.trim();
    const { ctx } = this;
    if (this.mode !== 'point') {
      // A lock's field: its number locks it, and the point is placed once its length and direction are both held;
      // left empty, the point the locks hold is placed, the cursor giving what is not locked (AutoCAD's Enter).
      if (text) {
        if (!this.lockTyped(text)) return;
        this.close();
        const s = ctx.settings.locks.value;
        if (s.length !== null && s.toward !== null) this.placeLocked();
        return;
      }
      this.close();
      this.placeLocked();
      return;
    }
    this.close();
    if (!text) return void ctx.commands.execute('tool.confirm');
    ctx.log.command(echo(text));
    // #ad: a point's place by its name (docs/adr/0152 §4).
    if (namedPoint(ctx, text)) return;
    // <45: the next point's direction locked (docs/adr/0166 §6).
    if (typedLock(ctx, text)) return;
    const f = ctx.format;
    if (!ctx.tools.active.input?.(text)) ctx.log.warn(`“${text}” anlaşılamadı. Mesafe, ${f.pairLabel}, ${f.relativeLabel} ya da ${f.polarLabel} yazın.`);
  }

  /** The point the locks hold, given to the tool as if clicked; the cursor gives what is not locked. */
  private placeLocked(): void {
    const { ctx } = this;
    const s = ctx.settings.locks.value;
    const from = lockReference(ctx);
    if (!from || !hasLocks(s)) return;
    const dir = lockedDirection(s, lockTravel(ctx), ctx.format.angles);
    const p = lockPoint(from, ctx.view.cursorWorld.value ?? from, s.length, dir?.u ?? null, dir?.both ?? false);
    if (p) ctx.tools.active.acceptPoint?.(p);
  }

  private close(refocus = true): void {
    if (!this.open) return;
    this.open = false;
    this.el.hidden = true;
    this.ctx.settings.valueCard.set(false);
    this.ctx.view.requestOverlay();
    if (refocus) this.ctx.view.focus();
  }
}
