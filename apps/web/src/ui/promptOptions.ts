import type { AppContext } from '../app/context';
import { h } from './dom';

/**
 * Tool prompts follow one convention: "Araç: adım [Seçenek (TUŞ) / Seçenek
 * (TUŞ): değer; not]". This turns them into parts the UI can render, so
 * every option is also a button (mouse users never need to type a letter).
 */
export interface PromptOption {
  label: string;
  /** What typing would send: a letter/code for tool.input, or Enter / Esc. */
  key: string;
  /** Current state of a toggle or value option ("açık", "Çizgili 45°"). */
  value?: string;
}

export interface ParsedPrompt {
  tool: string;
  step: string;
  options: PromptOption[];
  /** Bracketed text that is not an option (current distance, hints). */
  notes: string[];
}

const OPTION = /^(.+?)\s*\(([^()]+)\)\s*(?::\s*(.+))?$/;

export function parsePrompt(prompt: string): ParsedPrompt {
  const bracket = prompt.indexOf('[');
  const colon = prompt.indexOf(':');
  const hasTool = colon > 0 && (bracket < 0 || colon < bracket);
  const tool = hasTool ? prompt.slice(0, colon).trim() : '';
  let rest = hasTool ? prompt.slice(colon + 1) : prompt;
  const options: PromptOption[] = [];
  const notes: string[] = [];
  rest = rest.replace(/\[([^\]]*)\]/g, (_, inner: string) => {
    for (const raw of inner.split(/\s+\/\s+|\s*;\s*/)) {
      const part = raw.trim();
      if (!part) continue;
      const m = part.match(OPTION);
      if (m) options.push({ label: m[1].trim(), key: m[2].trim(), ...(m[3] && { value: m[3].trim() }) });
      else notes.push(part);
    }
    return ' ';
  });
  return { tool, step: rest.replace(/\s+/g, ' ').trim(), options, notes };
}

/** Sends an option as if it had been typed (Enter and Esc map to confirm and cancel). */
const TR_FOLD: Record<string, string> = { Ç: 'C', Ş: 'S', Ğ: 'G', Ö: 'O', Ü: 'U', İ: 'I' };
const fold = (k: string) => {
  const u = k.toLocaleUpperCase('tr-TR');
  return TR_FOLD[u] ?? u;
};

/**
 * The option a pressed letter selects: the exact key first, then the same
 * letter without Turkish marks, so "Çap (Ç)" also answers to C on a
 * keyboard without Ç.
 */
export function optionForKey(options: readonly PromptOption[], key: string): PromptOption | undefined {
  const k = key.toLocaleUpperCase('tr-TR');
  return options.find((o) => o.key.toLocaleUpperCase('tr-TR') === k) ?? options.find((o) => fold(o.key) === fold(k));
}

export function runPromptOption(ctx: AppContext, key: string): void {
  if (key === 'Enter') ctx.commands.execute('tool.confirm');
  else if (key === 'Esc') ctx.commands.execute('tool.cancel');
  else {
    ctx.log.command(`› ${key}`);
    if (!ctx.tools.active.input?.(key)) ctx.log.warn(`“${key}” seçeneği şu adımda kullanılamıyor.`);
  }
  ctx.view.focus();
}

/** Option buttons: label, current value and the key that does the same. */
export function optionButtons(ctx: AppContext, options: readonly PromptOption[], cls: string): HTMLElement[] {
  return options.map((o) => {
    const b = h(
      'button',
      { class: cls, type: 'button', title: `${o.label}${o.value ? `: ${o.value}` : ''} (${o.key})` },
      h('span', null, o.label),
      o.value ? h('span', { class: `${cls}-value` }, o.value) : null,
      h('kbd', { class: `${cls}-key` }, o.key),
    );
    // Keep focus where it is (the drawing) and act on click.
    b.addEventListener('pointerdown', (e) => e.preventDefault());
    b.addEventListener('click', () => runPromptOption(ctx, o.key));
    return b;
  });
}
