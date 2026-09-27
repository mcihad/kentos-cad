import { describe, expect, it } from 'vitest';
import { menuRowLook } from '../../app/menus';
import type { ShellKind } from '../../app/state';
import { LINE_TYPE_LABEL } from '../../model/layers';
import { TOOL_GROUP_LABEL } from '../../tools/Tool';
import { DRAW_COLORS, LINE_WEIGHTS, PLOT_SCALES, weightText } from '../toolbar/fields';
import {
  MENUBAR_FOLD,
  SHELL_TEXTS,
  TOOLBAR_FOLD,
  TOOLBAR_GROUPS,
  TOOLBAR_WIDTHS,
  TOOLBOX,
  TOOLBOX_GROUPS,
  VIEW_MORE,
  fitColumns,
  toolboxColumns,
  toolboxPlace,
  toolboxShown,
  undockAt,
} from './shellPlan';

/**
 * The classic shell's rules (fixtures/shell/v1/shell.json, format in
 * fixtures/shell/README.md): the menu bar's and the toolbar's fold steps,
 * the toolbar's groups and fields, the toolbox's groups, place, columns and
 * showing, a command's row in a menu, and the bars' words. The file's
 * answers are worked out apart from this code (scripts/fixtures/shell_cases.py);
 * the desktop's classic shell checks itself against the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/shell/v1/shell.json', { query: '?raw', import: 'default', eager: true });
type Place = { title: string; x: number; y: number; w: number; h: number; hostW: number; hostH: number; expect: { x: number; y: number } };
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  menubar: { folds: unknown; texts: { label: string; dirty: string; crs: string; crsTip: { sample: number; text: string } } };
  toolbar: { groups: unknown; viewMore: unknown; widths: unknown; folds: unknown; texts: unknown };
  fields: { byLayer: string; colors: unknown; lineTypes: unknown; weights: { mm: number; text: string }[]; scales: { denominator: number; text: string }[] };
  toolbox: {
    groups: { id: string; label: string }[];
    constants: unknown;
    texts: Record<string, string | { sample: string; text: string }>;
    columns: { stored: number; columns: number }[];
    fit: { base: number; fitsFrom: number | null; columns: number }[];
    place: Place[];
    undock: { x: number; y: number; expect: unknown }[];
    shown: { shell: ShellKind; visible: boolean; ribbonToolbox: boolean; shown: boolean }[];
  };
  menuRows: { id: string; checked: boolean | null; expect: { icon: boolean; checked: boolean | null; radio: boolean } }[];
};

const plain = (v: unknown): unknown => JSON.parse(JSON.stringify(v));

describe('classic shell rules (fixtures/shell/v1/shell.json)', () => {
  it('is a v1 shell file', () => {
    expect([F.format, F.version]).toEqual(['kentos.shell', 1]);
  });

  it('makes room in the menu bar step by step, and says its words', () => {
    expect(plain(MENUBAR_FOLD)).toEqual(F.menubar.folds);
    const t = SHELL_TEXTS.menubar;
    expect({ label: t.label, dirty: t.dirty, crs: t.crs, crsTip: { sample: F.menubar.texts.crsTip.sample, text: t.crsTip(F.menubar.texts.crsTip.sample) } }).toEqual(F.menubar.texts);
  });

  it('lays out the toolbar’s groups and folds it step by step', () => {
    expect(plain(TOOLBAR_GROUPS)).toEqual(F.toolbar.groups);
    expect(plain([VIEW_MORE, TOOLBAR_WIDTHS, TOOLBAR_FOLD, SHELL_TEXTS.toolbar])).toEqual([F.toolbar.viewMore, F.toolbar.widths, F.toolbar.folds, F.toolbar.texts]);
  });

  it('offers the current-property fields’ values', () => {
    expect(plain(DRAW_COLORS)).toEqual(F.fields.colors);
    expect(LINE_TYPE_LABEL).toEqual(F.fields.lineTypes);
    expect(LINE_WEIGHTS.map((mm) => ({ mm, text: weightText(mm) }))).toEqual(F.fields.weights);
    expect(PLOT_SCALES.map((denominator) => ({ denominator, text: `1:${denominator}` }))).toEqual(F.fields.scales);
  });

  it('names the toolbox’s groups in order, with its constants and words', () => {
    expect(TOOLBOX_GROUPS.map((id) => ({ id, label: TOOL_GROUP_LABEL[id] }))).toEqual(F.toolbox.groups);
    expect(plain(TOOLBOX)).toEqual(F.toolbox.constants);
    const t = SHELL_TEXTS.toolbox;
    const sample = (F.toolbox.texts.fold as { sample: string }).sample;
    expect({ ...t, fold: { sample, text: t.fold(sample) }, unfold: { sample, text: t.unfold(sample) } }).toEqual(F.toolbox.texts);
  });

  it('reads the stored columns, widens to fit, places, undocks and shows the toolbox', () => {
    for (const c of F.toolbox.columns) expect(toolboxColumns(c.stored), String(c.stored)).toBe(c.columns);
    for (const c of F.toolbox.fit) expect(fitColumns(c.base, (cols) => c.fitsFrom !== null && cols >= c.fitsFrom), JSON.stringify(c)).toBe(c.columns);
    for (const c of F.toolbox.place) expect(toolboxPlace(c.x, c.y, c.w, c.h, c.hostW, c.hostH), c.title).toEqual(c.expect);
    for (const c of F.toolbox.undock) expect(undockAt(c.x, c.y)).toEqual(c.expect);
    for (const c of F.toolbox.shown) expect(toolboxShown(c.shell, c.visible, c.ribbonToolbox), JSON.stringify(c)).toBe(c.shown);
  });

  it('draws a command’s row in a menu: a check for a toggle, a radio for a choice, a tool as an action', () => {
    for (const c of F.menuRows) {
      const look = menuRowLook(c.id, c.checked ?? undefined);
      expect({ ...look, checked: look.checked ?? null }, `${c.id} ${c.checked}`).toEqual(c.expect);
    }
  });
});
