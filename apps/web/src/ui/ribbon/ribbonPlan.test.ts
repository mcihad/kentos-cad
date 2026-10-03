import { describe, expect, it } from 'vitest';
import { menuRowLook } from '../../app/menus';
import { QUICK_ACCESS, ribbonTabs, type SplitEntry } from '../../app/ribbon';
import { workspaceById, workspaceFilter } from '../../app/workspaces';
import { BUILTIN_TOOLS } from '../../processing/builtin';
import { BUILTIN_MODELS } from '../../processing/builtin/models';
import { ProcessingRegistry } from '../../processing/registry';
import { LINE_TYPE_LABEL } from '../../model/layers';
import { TOOL_CATALOG } from '../../tools/catalog';
import { DRAW_COLORS, LINE_WEIGHTS, PLOT_SCALES, weightText } from './fields';
import { assignKeyTips, firstLevelTips, keyTipStep, lettersOf } from './keytips';
import { commandMenu, QUICK_ACCESS_OFFERS, quickAccessMenu, RIBBON_TEXTS, ribbonMenu, splitFace, splitMenu, withQuickAccess } from './ribbonPlan';

/**
 * The ribbon's rules (fixtures/shell/v1/ribbon.json, format in
 * fixtures/shell/README.md; docs/specs/ribbon.md): the key tips' letters,
 * levels and keys, the quick access bar's menu and changes, the right click
 * menus, a split button's list, the Özellikler panel's lists and a
 * command's row in a menu. The file's answers are worked out apart
 * from this code (scripts/fixtures/ribbon_cases.py); its tab names are held
 * to the ribbon the web builds in each work mode.
 */

const files = import.meta.glob<string>('../../../../../fixtures/shell/v1/ribbon.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  texts: Record<string, unknown>;
  quickAccessFixed: string[];
  quickAccessOffers: string[];
  tabs: Record<'cad' | 'gis', { id: string; label: string; contextual?: boolean }[]>;
  keyTips: {
    letters: { label: string; letters: string }[];
    assign: { title: string; labels: string[]; reserved: string[]; tips: string[] }[];
    firstLevel: { mode: string; selection: boolean; quickAccess: number; tabLabels: string[]; tips: unknown }[];
    steps: { level: 'tabs' | 'controls'; typed: string; key: string; ctrl?: boolean; tips: string[]; step: unknown }[];
  };
  quickAccessMenus: { title: string; bar: string[]; exists: string[]; rows: unknown }[];
  toggles: { kept: string[]; command: string; on: boolean; result: string[] }[];
  commandMenus: { command: string; bar: string[]; rows: unknown }[];
  ribbonMenu: unknown;
  splitMenus: { entries: SplitEntry[]; menu: unknown }[];
  splitFaces: { entry: SplitEntry; face: unknown }[];
  fields: { byLayer: string; colors: unknown; lineTypes: unknown; weights: { mm: number; text: string }[]; scales: { denominator: number; text: string }[] };
  menuRows: { id: string; checked: boolean | null; expect: { icon: boolean; checked: boolean | null; radio: boolean } }[];
};

const plain = (v: unknown): unknown => JSON.parse(JSON.stringify(v));

/** RIBBON_TEXTS as the file writes them: a text made from values as `{ sample: [...], text }`. */
function fileTexts(samples: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(
    Object.entries(RIBBON_TEXTS).map(([k, v]) => {
      if (typeof v !== 'function') return [k, v];
      const sample = (samples[k] as { sample?: unknown[] } | undefined)?.sample;
      if (!sample) throw new Error(`Metin örneği yok: ${k}`);
      return [k, { sample, text: (v as (...a: unknown[]) => string)(...sample) }];
    }),
  );
}

const registry = new ProcessingRegistry();
BUILTIN_TOOLS.forEach((t) => registry.register(t));

describe('the ribbon (fixtures/shell/v1/ribbon.json)', () => {
  it('is a v1 ribbon file with the ribbon’s words, the bar’s fixed commands and offers, and the web’s tabs in each work mode', () => {
    expect([F.format, F.version]).toEqual(['kentos.ribbon', 1]);
    expect(fileTexts(F.texts)).toEqual(F.texts);
    expect([QUICK_ACCESS, QUICK_ACCESS_OFFERS]).toEqual([F.quickAccessFixed, F.quickAccessOffers]);
    for (const mode of ['cad', 'gis'] as const) {
      const tabs = ribbonTabs({ tools: TOOL_CATALOG, processing: registry.tree(), models: BUILTIN_MODELS, iconOf: () => undefined, filter: workspaceFilter(workspaceById(mode), TOOL_CATALOG) });
      expect(
        tabs.map((t) => ({ id: t.id, label: t.label, ...(t.contextual ? { contextual: true } : {}) })),
        mode,
      ).toEqual(F.tabs[mode]);
    }
  });

  it('gives the key tips their letters: one where a first letter is free, two otherwise; digits on the bar first', () => {
    for (const c of F.keyTips.letters) expect(lettersOf(c.label), c.label).toBe(c.letters);
    for (const c of F.keyTips.assign) expect(assignKeyTips(c.labels, new Set(c.reserved)), c.title).toEqual(c.tips);
    for (const c of F.keyTips.firstLevel) expect(firstLevelTips(c.quickAccess, c.tabLabels), `${c.mode}, ${c.quickAccess} on the bar`).toEqual(c.tips);
  });

  it('answers each key while the tips show', () => {
    for (const c of F.keyTips.steps) expect(keyTipStep(c.level, c.typed, c.tips, c.key, { ctrl: c.ctrl }), JSON.stringify([c.level, c.typed, c.key, c.ctrl])).toEqual(c.step);
  });

  it('builds the bar’s menu, adds and takes off commands, and offers the bar on a right click', () => {
    for (const c of F.quickAccessMenus) expect(quickAccessMenu(c.bar, (id) => c.exists.includes(id)), c.title).toEqual(c.rows);
    for (const c of F.toggles) expect(withQuickAccess(c.kept, c.command, c.on), JSON.stringify(c)).toEqual(c.result);
    for (const c of F.commandMenus) expect(commandMenu(c.command, c.bar), JSON.stringify(c)).toEqual(c.rows);
    expect(ribbonMenu()).toEqual(F.ribbonMenu);
  });

  it('lists a split button’s choices and names its top', () => {
    for (const c of F.splitMenus) expect(splitMenu(c.entries), c.entries[0].title).toEqual(c.menu);
    for (const c of F.splitFaces) expect(splitFace(c.entry), JSON.stringify(c.entry)).toEqual(c.face);
  });

  it('offers the Özellikler panel’s values', () => {
    expect(plain(DRAW_COLORS)).toEqual(F.fields.colors);
    expect(LINE_TYPE_LABEL).toEqual(F.fields.lineTypes);
    expect(LINE_WEIGHTS.map((mm) => ({ mm, text: weightText(mm) }))).toEqual(F.fields.weights);
    expect(PLOT_SCALES.map((denominator) => ({ denominator, text: `1:${denominator}` }))).toEqual(F.fields.scales);
  });

  it('draws a command’s row in a menu: a check for a toggle, a radio for a choice, a tool as an action', () => {
    for (const c of F.menuRows) {
      const look = menuRowLook(c.id, c.checked ?? undefined);
      expect({ ...look, checked: look.checked ?? null }, `${c.id} ${c.checked}`).toEqual(c.expect);
    }
  });
});
