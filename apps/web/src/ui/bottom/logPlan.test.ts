import { describe, expect, it } from 'vitest';
import { LOG_LIMIT, MessageLog, type LogLevel } from '../../app/state';
import panelsCss from '../../styles/panels.css?raw';
import shellCss from '../../styles/shell.css?raw';
import { BOTTOM_TABS, BOTTOM_TEXTS, FOLLOW_WITHIN, ICON_SIZE, LEVEL_ICON, echo, flashOf, listedIn, logTime } from './logPlan';
import { seenNow, unseenWarnings } from './warnings';

/**
 * How the log is written (fixtures/shell/v1/log.json, format in
 * fixtures/shell/README.md): a line's time and level, which lines each tab
 * lists, the Uyarılar badge, how many lines are kept, the echo, the status
 * bar's message, and the rows' look as the style sheets draw it. The file's
 * answers are worked out apart from this code (scripts/fixtures/log_cases.py);
 * the desktop's bottom panel and status bar check themselves against it.
 */

const files = import.meta.glob<string>('../../../../../fixtures/shell/v1/log.json', { query: '?raw', import: 'default', eager: true });
type Step = string | { push: LogLevel; times: number };
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  timeZone: string;
  tabs: unknown;
  texts: unknown;
  limit: number;
  followWithin: number;
  levels: { level: LogLevel; icon: string | null; listedIn: ('history' | 'messages')[] }[];
  look: Look;
  times: { title: string; at: number; text: string }[];
  echo: { typed: string; text: string }[];
  flash: { level: LogLevel; text: string; shown: unknown }[];
  kept: { pushed: number; length: number; first: number; last: number }[];
  badge: { title: string; steps: { step: Step; count: number; lines: number }[] }[];
};

// Times are the device's local time; the cases fix the zone (Node reads TZ again when it changes).
(globalThis as unknown as { process: { env: Record<string, string> } }).process.env.TZ = F.timeZone;

type Tone = string | null;
interface Look {
  list: { font: string; size: string; weight: number; lineHeight: number; padding: number[] };
  row: { columns: number[]; padding: number[]; hover: Tone };
  time: Tone;
  iconTop: number;
  iconSize: number;
  text: { tone: Tone; wrap: string };
  levels: Record<LogLevel, { icon: Tone; text: Tone; weight: number }>;
  flash: { gap: number; padding: number[]; fadeMs: number; icon: Record<'info' | 'success' | 'warn' | 'error', Tone> };
}

/** A style sheet's rules: selector → property → value (comments dropped, grouped selectors split). */
function rules(css: string): Map<string, Map<string, string>> {
  const out = new Map<string, Map<string, string>>();
  for (const m of css.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const decls = [...m[2].matchAll(/([\w-]+)\s*:\s*([^;]+);/g)].map((d): [string, string] => [d[1], d[2].trim()]);
    for (const sel of m[1].split(',')) {
      const key = sel.trim().replace(/\s+/g, ' ');
      out.set(key, new Map([...(out.get(key) ?? []), ...decls]));
    }
  }
  return out;
}
const tone = (v: string | undefined): Tone => (v ? (/^var\(--c-([\w-]+)\)$/.exec(v)?.[1] ?? `?${v}`) : null);
const px = (v: string | undefined): number[] => (v ?? '').split(/\s+/).map((p) => Number.parseFloat(p));

/** The look as panels.css and shell.css draw it, in the file's terms. */
function lookOfSheets(): Look {
  const panels = rules(panelsCss);
  const shell = rules(shellCss);
  const get = (m: Map<string, Map<string, string>>, sel: string, prop: string) => m.get(sel)?.get(prop);
  const font = /^(\d+) var\(--fs-(\w+)\) \/ ([\d.]+) var\(--font-(\w+)\)$/.exec(get(panels, '.log', 'font') ?? '');
  const listWeight = Number(font?.[1]);
  const level = (l: LogLevel) => ({
    icon: tone(get(panels, `.log__row--${l} .log__icon`, 'color')),
    text: tone(get(panels, `.log__row--${l} .log__text`, 'color') ?? get(panels, '.log__text', 'color')),
    weight: Number(get(panels, `.log__row--${l} .log__text`, 'font-weight') ?? listWeight),
  });
  const flashIcon = (l: string) => tone(get(shell, `.status__flash[data-level='${l}'] .icon`, 'color'));
  return {
    list: { font: font?.[4] ?? '?', size: font?.[2] ?? '?', weight: listWeight, lineHeight: Number(font?.[3]), padding: px(get(panels, '.log', 'padding')) },
    row: {
      // Fixed columns (time, icon); the text takes the rest (1fr).
      columns: (get(panels, '.log__row', 'grid-template-columns') ?? '')
        .split(/\s+/)
        .filter((t) => t.endsWith('px'))
        .map((t) => Number.parseFloat(t)),
      padding: px(get(panels, '.log__row', 'padding')),
      hover: tone(get(panels, '.log__row:hover', 'background')),
    },
    time: tone(get(panels, '.log__time', 'color')),
    iconTop: px(get(panels, '.log__icon', 'padding-top'))[0],
    iconSize: ICON_SIZE,
    text: { tone: tone(get(panels, '.log__text', 'color')), wrap: get(panels, '.log__text', 'white-space') ?? '?' },
    levels: { command: level('command'), info: level('info'), success: level('success'), warn: level('warn'), error: level('error') },
    flash: {
      gap: px(get(shell, '.status__flash', 'gap'))[0],
      padding: px(get(shell, '.status__flash', 'padding')),
      fadeMs: Number(/^opacity (\d+)ms$/.exec(get(shell, '.status__flash', 'transition') ?? '')?.[1]),
      icon: { info: flashIcon('info'), success: flashIcon('success'), warn: flashIcon('warn'), error: flashIcon('error') },
    },
  };
}

describe('the log (fixtures/shell/v1/log.json)', () => {
  it('is a v1 log file with the panel’s tabs, words and limits', () => {
    expect([F.format, F.version]).toEqual(['kentos.log', 1]);
    expect(BOTTOM_TABS).toEqual(F.tabs);
    expect(BOTTOM_TEXTS).toEqual(F.texts);
    expect([LOG_LIMIT, FOLLOW_WITHIN]).toEqual([F.limit, F.followWithin]);
  });

  it('writes a line’s time as the local clock shows it', () => {
    for (const c of F.times) expect(logTime(new Date(c.at)), c.title).toBe(c.text);
  });

  it('gives each level its icon and its tabs', () => {
    expect(F.levels.map((l) => l.level)).toEqual(Object.keys(LEVEL_ICON));
    for (const l of F.levels) {
      expect(LEVEL_ICON[l.level], l.level).toBe(l.icon);
      expect((['history', 'messages'] as const).filter((t) => listedIn(t, l.level)), l.level).toEqual(l.listedIn);
    }
  });

  it('echoes what was typed, and shows in the status bar only what it should, for as long as it should', () => {
    for (const c of F.echo) expect(echo(c.typed)).toBe(c.text);
    for (const c of F.flash) expect(flashOf(c.level, c.text), `${c.level}: ${c.text}`).toEqual(c.shown);
  });

  it('keeps the newest lines, and counts the warnings the Uyarılar tab has not shown', () => {
    for (const c of F.kept) {
      const log = new MessageLog();
      for (let i = 0; i < c.pushed; i++) log.info(`satır ${i + 1}`);
      const ids = log.entries.value.map((e) => e.id);
      expect([ids.length, ids[0], ids.at(-1)], String(c.pushed)).toEqual([c.length, c.first, c.last]);
    }
    for (const b of F.badge) {
      const log = new MessageLog();
      let seen = 0;
      for (const s of b.steps) {
        const step = s.step;
        if (step === 'look') seen = seenNow(log.entries.value, seen, true);
        else if (step === 'clear') log.clear();
        else if (typeof step === 'string') log.push(step as LogLevel, step);
        else for (let i = 0; i < step.times; i++) log.push(step.push, `${step.push} ${i + 1}`);
        expect([unseenWarnings(log.entries.value, seen), log.entries.value.length], `${b.title}: ${JSON.stringify(step)}`).toEqual([s.count, s.lines]);
      }
    }
  });

  it('draws the rows and the status bar’s message as the file says (panels.css, shell.css)', () => {
    expect(lookOfSheets()).toEqual(F.look);
  });
});
