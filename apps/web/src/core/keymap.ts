import type { CommandRegistry } from './commands';
import { listen, type Disposable } from './disposable';

/**
 * Chords are written "Ctrl+Shift+Z", "Alt+P", "L", "F3", "+".
 * Letters follow the character the layout produces, so "L" is the key
 * labelled L on Turkish Q, Turkish F and US keyboards alike (ı and i both
 * count as I). When a modifier turns the key into a symbol, the physical
 * key position (KeyboardEvent.code) is used instead.
 */
export interface Binding {
  chord: string;
  command: string;
  args?: unknown;
  /** Extra runtime condition (e.g. "only when a tool is active"). */
  when?: () => boolean;
  /** Fire even when focus is inside a text field. */
  allowInInput?: boolean;
}

const MOD_ORDER = ['Ctrl', 'Alt', 'Shift'] as const;

const NAMED_CODES: Record<string, string> = {
  Escape: 'Esc',
  Enter: 'Enter',
  NumpadEnter: 'Enter',
  Space: 'Space',
  Delete: 'Delete',
  Backspace: 'Backspace',
  Tab: 'Tab',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  NumpadAdd: '+',
  NumpadSubtract: '-',
};

export function keyFromEvent(e: KeyboardEvent): string | null {
  const c = e.code;
  const ch = e.key.length === 1 ? (e.key === 'ı' ? 'I' : e.key.toUpperCase()) : '';
  if (/^[A-Z]$/.test(ch)) return ch;
  if (c.startsWith('Key')) return c.slice(3);
  if (c.startsWith('Digit')) return c.slice(5);
  if (/^F\d{1,2}$/.test(c)) return c;
  if (NAMED_CODES[c]) return NAMED_CODES[c];
  // Symbol keys differ per layout: trust the produced character.
  if (e.key === '+' || e.key === '-' || e.key === ',') return e.key;
  return null;
}

export function chordFromEvent(e: KeyboardEvent): string | null {
  if (e.key === 'Control' || e.key === 'Shift' || e.key === 'Alt' || e.key === 'Meta') return null;
  const key = keyFromEvent(e);
  if (!key) return null;
  const mods: string[] = [];
  if (e.ctrlKey || e.metaKey) mods.push('Ctrl');
  if (e.altKey) mods.push('Alt');
  if (e.shiftKey && key !== '+' && key !== '-') mods.push('Shift');
  return [...mods, key].join('+');
}

function splitChord(chord: string): string[] {
  if (chord === '+') return ['+'];
  if (chord.endsWith('++')) return [...chord.slice(0, -2).split('+'), '+'];
  return chord.split('+');
}

export function normalizeChord(chord: string): string {
  const parts = splitChord(chord.trim());
  const key = parts.pop()!;
  const mods = parts.map((m) => {
    const l = m.toLowerCase();
    return l === 'ctrl' || l === 'cmd' || l === 'mod' ? 'Ctrl' : l === 'alt' ? 'Alt' : 'Shift';
  });
  const ordered = MOD_ORDER.filter((m) => mods.includes(m));
  const k = key.length === 1 ? key.toUpperCase() : key[0].toUpperCase() + key.slice(1);
  return [...ordered, k].join('+');
}

/** Human-readable form for menus and tooltips. */
export function formatChord(chord: string): string {
  return splitChord(chord)
    .map((p) => (p === 'Delete' ? 'Del' : p === 'Space' ? 'Boşluk' : p))
    .join('+');
}

/** Very short form for key caps on tool buttons: "L", "⇧M", "⌥P". */
export function formatChordCompact(chord: string): string {
  const parts = splitChord(chord);
  const key = parts.pop()!;
  const pre = parts.map((m) => (m === 'Shift' ? '⇧' : m === 'Alt' ? '⌥' : '⌃')).join('');
  return pre + (key === 'Delete' ? 'Del' : key === 'Esc' ? 'Esc' : key);
}

export function isTextInput(el: Element | null): boolean {
  if (!el) return false;
  if (el instanceof HTMLInputElement) {
    return !['checkbox', 'radio', 'button', 'range', 'color'].includes(el.type);
  }
  return el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement || (el as HTMLElement).isContentEditable;
}

export class Keymap {
  private byChord = new Map<string, Binding[]>();
  private byCommand = new Map<string, Binding[]>();
  private readonly commands: CommandRegistry;
  /** Called for keys no binding handled (used by the command line). */
  fallback: ((e: KeyboardEvent) => void) | null = null;
  /**
   * Consulted before any binding; returning true consumes the key. The UI
   * uses it for a running command's option letters, which beat tool shortcuts.
   */
  intercept: ((e: KeyboardEvent) => boolean) | null = null;

  constructor(commands: CommandRegistry) {
    this.commands = commands;
  }

  bind(chord: string, command: string, opts: Omit<Binding, 'chord' | 'command'> = {}): Disposable {
    const b: Binding = { chord: normalizeChord(chord), command, ...opts };
    const push = (map: Map<string, Binding[]>, key: string) => {
      const list = map.get(key) ?? [];
      list.unshift(b); // later bindings win
      map.set(key, list);
    };
    push(this.byChord, b.chord);
    push(this.byCommand, command);
    return () => {
      this.byChord.set(b.chord, (this.byChord.get(b.chord) ?? []).filter((x) => x !== b));
      this.byCommand.set(command, (this.byCommand.get(command) ?? []).filter((x) => x !== b));
    };
  }

  /** Primary chord for a command, for display. */
  chordFor(command: string): string | undefined {
    return this.byCommand.get(command)?.at(-1)?.chord;
  }

  all(): Binding[] {
    return [...this.byCommand.values()].flatMap((l) => [...l].reverse());
  }

  resolve(e: KeyboardEvent): Binding | undefined {
    const chord = chordFromEvent(e);
    if (!chord) return undefined;
    const focused = document.activeElement;
    // Enter/Space keep their native meaning on buttons, tree rows and menus.
    if ((chord === 'Enter' || chord === 'Space') && focused instanceof HTMLElement && focused.matches('button, [role], a[href]')) return undefined;
    const inInput = isTextInput(focused);
    return this.byChord
      .get(chord)
      ?.find((b) => (!inInput || b.allowInInput) && (b.when?.() ?? true) && this.commands.isEnabled(b.command));
  }

  attach(target: Window): Disposable {
    return listen<KeyboardEvent>(target, 'keydown', (e) => {
      if (e.defaultPrevented || e.isComposing) return;
      if (this.intercept?.(e)) {
        e.preventDefault();
        return;
      }
      const b = this.resolve(e);
      if (b) {
        e.preventDefault();
        if (!e.repeat) this.commands.execute(b.command, b.args);
        return;
      }
      this.fallback?.(e);
    });
  }
}
