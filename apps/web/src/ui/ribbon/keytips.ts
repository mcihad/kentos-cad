import { h, overlayRoot } from '../dom';
import { DisposableStore, listen } from '../../core/disposable';

/**
 * Keyboard letters over the ribbon (KeyTips, as in Office and AutoCAD):
 * Alt tapped alone or F6 shows a letter on every tab and a digit on the
 * quick access buttons; typing a tab's letter opens it and shows one or two
 * letters on each of its controls; typing those runs the control. Esc goes
 * back a level, then away; a click anywhere or any other key leaves.
 *
 * Letters come from the labels (Turkish letters folded, as on the command
 * line), first letters first, so they stay the same while the ribbon does.
 */

/** What the key tips read from and do in the ribbon. */
export interface KeyTipsHost {
  /** Visible tabs: their id, label and button. */
  tabs(): { id: string; label: string; el: HTMLElement }[];
  /** Quick access buttons, in order. */
  quickAccess(): HTMLElement[];
  /** Opens a tab (over the drawing when the ribbon is folded). */
  openTab(id: string): void;
  /** The open tab's controls that can be used now, in reading order. */
  controls(): HTMLElement[];
  /** The ribbon's own element (clicks inside it do not count as "elsewhere" until a control runs). */
  readonly root: HTMLElement;
}

const FOLD: Record<string, string> = { ç: 'C', ğ: 'G', ı: 'I', i: 'I', ö: 'O', ş: 'S', ü: 'U', â: 'A', î: 'I', û: 'U' };

/** Letters of a label, upper case, Turkish letters folded, others dropped. */
export function lettersOf(label: string): string {
  let out = '';
  for (const ch of label) {
    const lower = ch.toLocaleLowerCase('tr');
    const up = FOLD[lower] ?? lower.toUpperCase();
    if (/^[A-Z0-9]$/.test(up)) out += up;
  }
  return out;
}

/** The words' first letters of a label (Yeni proje → YP). */
function initials(label: string): string {
  return label
    .split(/[\s/–-]+/)
    .map((w) => lettersOf(w)[0] ?? '')
    .join('');
}

/**
 * Unique key tips for labels: a single letter where one is free (the
 * label's first letter first, then its other letters), otherwise two
 * (initials, then first letter and another). Tips never start with
 * another tip, so a typed letter is never ambiguous.
 */
export function assignKeyTips(labels: readonly string[], reserved: ReadonlySet<string> = new Set()): string[] {
  const used = new Set(reserved);
  const out: string[] = new Array(labels.length).fill('');
  const singles = new Set<string>();
  // Pass 1: one letter for labels whose first letter nobody else starts with.
  const firsts = labels.map((l) => lettersOf(l)[0] ?? '');
  const counts = new Map<string, number>();
  for (const f of firsts) counts.set(f, (counts.get(f) ?? 0) + 1);
  labels.forEach((_, i) => {
    const f = firsts[i];
    if (f && counts.get(f) === 1 && !used.has(f)) {
      out[i] = f;
      used.add(f);
      singles.add(f);
    }
  });
  // Pass 2: two letters for the rest, never starting with a single tip.
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  labels.forEach((label, i) => {
    if (out[i]) return;
    const letters = lettersOf(label) || 'X';
    const ini = initials(label);
    const candidates = [ini.length >= 2 ? ini.slice(0, 2) : '', ...[...letters.slice(1)].map((c) => letters[0] + c), ...[...alphabet].map((c) => letters[0] + c)];
    const starts = [...(letters[0] && !singles.has(letters[0]) ? [letters[0]] : []), ...[...alphabet].filter((c) => !singles.has(c))];
    let tip = candidates.find((c) => c.length === 2 && !used.has(c) && !singles.has(c[0]));
    if (!tip) for (const s of starts) for (const c of alphabet) if (!tip && !used.has(s + c)) tip = s + c;
    out[i] = tip ?? '';
    if (tip) used.add(tip);
  });
  return out;
}

/** Label of a control: its aria-label, else its text. */
function labelOf(el: HTMLElement): string {
  return el.getAttribute('aria-label') ?? el.textContent ?? '';
}

type Level = { kind: 'tabs' } | { kind: 'controls'; tab: string };

export class KeyTips {
  private readonly host: KeyTipsHost;
  private layer: HTMLElement | null = null;
  private level: Level | null = null;
  private typed = '';
  private tips: { tip: string; el: HTMLElement; badge: HTMLElement; run: () => void }[] = [];
  private readonly live = new DisposableStore();

  constructor(host: KeyTipsHost) {
    this.host = host;
  }

  get shown(): boolean {
    return this.level !== null;
  }

  /** Shows the first level (tabs and quick access). */
  show(): void {
    this.hide();
    this.level = { kind: 'tabs' };
    this.layer = h('div', { class: 'keytips', 'aria-hidden': 'true' });
    overlayRoot().append(this.layer);
    // Captured, so the keys go to the tips before the drawing, the command line or a shortcut.
    this.live.add(listen<KeyboardEvent>(window, 'keydown', (e) => this.key(e), true));
    this.live.add(listen(window, 'pointerdown', () => this.hide(), true));
    this.live.add(listen(window, 'blur', () => this.hide()));
    this.live.add(listen(window, 'resize', () => this.hide()));
    this.render();
  }

  hide(): void {
    this.level = null;
    this.typed = '';
    this.tips = [];
    this.live.dispose();
    this.layer?.remove();
    this.layer = null;
  }

  private render(): void {
    if (!this.layer || !this.level) return;
    this.typed = '';
    const items: { label: string; el: HTMLElement; run: () => void }[] = [];
    let reserved = new Set<string>();
    const fixed: { tip: string; el: HTMLElement; run: () => void }[] = [];
    if (this.level.kind === 'tabs') {
      this.host.quickAccess().forEach((el, i) => i < 9 && fixed.push({ tip: String(i + 1), el, run: () => this.press(el) }));
      reserved = new Set(fixed.map((f) => f.tip));
      for (const t of this.host.tabs()) items.push({ label: t.label, el: t.el, run: () => this.openTab(t.id) });
    } else {
      for (const el of this.host.controls()) items.push({ label: labelOf(el), el, run: () => this.press(el) });
    }
    const tips = assignKeyTips(
      items.map((i) => i.label),
      reserved,
    );
    this.tips = [...fixed, ...items.map((it, i) => ({ tip: tips[i], el: it.el, run: it.run }))]
      .filter((t) => t.tip)
      .map((t) => ({ ...t, badge: this.badge(t.tip, t.el) }));
    this.layer.replaceChildren(...this.tips.map((t) => t.badge));
  }

  /**
   * Where a badge sits: under a tab; on a large button across its bottom edge, below the label; on a small or
   * icon button over its icon, so the label beside it stays readable.
   */
  private badge(tip: string, el: HTMLElement): HTMLElement {
    const r = el.getBoundingClientRect();
    const b = h('span', { class: 'keytips__tip', dataset: { tip } }, tip);
    const icon = el.querySelector('svg')?.getBoundingClientRect();
    let x: number;
    let y: number;
    if (this.level?.kind === 'tabs') [x, y] = [r.left + r.width / 2, r.bottom - 6];
    else if (r.height > 40) [x, y] = [r.left + r.width / 2, r.bottom - 7];
    else [x, y] = [icon ? icon.left + icon.width / 2 : r.left + 10, r.top + r.height / 2 - 2];
    b.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px) translateX(-50%)`;
    return b;
  }

  private openTab(id: string): void {
    this.host.openTab(id);
    this.level = { kind: 'controls', tab: id };
    // The tab lays out (and fits) in the next frame.
    requestAnimationFrame(() => this.render());
  }

  /** Runs a control as a click would; one that opens a menu opens it from the keyboard (its first item focused). */
  private press(el: HTMLElement): void {
    // A panel folded into one button opens under it: its controls get letters in turn.
    if (el.classList.contains('rpanel__collapsed')) {
      el.click();
      requestAnimationFrame(() => this.render());
      return;
    }
    this.hide();
    if (el.getAttribute('aria-haspopup')) {
      el.focus();
      el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
    } else el.click();
  }

  private key(e: KeyboardEvent): void {
    if (!this.level) return;
    if (e.key === 'Alt' || e.key === 'Shift') return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') {
      if (this.typed) {
        this.typed = '';
        this.mark();
      } else if (this.level.kind === 'controls') {
        this.level = { kind: 'tabs' };
        this.render();
      } else this.hide();
      return;
    }
    if (e.key === 'Backspace' && this.typed) {
      this.typed = this.typed.slice(0, -1);
      this.mark();
      return;
    }
    const ch = lettersOf(e.key);
    if (ch.length !== 1 || e.ctrlKey || e.metaKey) {
      this.hide();
      return;
    }
    const typed = this.typed + ch;
    const exact = this.tips.find((t) => t.tip === typed);
    if (exact) return exact.run();
    if (this.tips.some((t) => t.tip.startsWith(typed))) {
      this.typed = typed;
      this.mark();
    }
  }

  /** Dims the tips that no longer match what was typed. */
  private mark(): void {
    for (const t of this.tips) t.badge.toggleAttribute('data-off', !t.tip.startsWith(this.typed));
  }
}
