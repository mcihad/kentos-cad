import { describe, expect, it } from 'vitest';
import {
  BOTTOM_HEIGHT,
  DOCK_WIDTH,
  LAYERS_FRACTION,
  LAYOUT_DEFAULTS,
  LAYOUT_FIELDS,
  LAYOUT_KEY,
  bottomHeightOn,
  dockWidthOn,
  draggedLayersFraction,
  readLayout,
} from './layoutPlan';
import { QUICK_ACCESS, quickAccessOf, splitCurrent, startTab } from './ribbon';
import { SAVE_DELAY_MS } from './state';

/**
 * The workbench's kept layout (fixtures/shell/v1/layout.json, format in
 * fixtures/shell/README.md): what is kept and its defaults, how a stored
 * value is read, the migration, the sizes as the window allows them, the
 * dock's split dragged, and the ribbon's kept tab, quick access bar and
 * split choices. The file's answers are worked out apart from this code
 * (scripts/fixtures/layout_cases.py); the desktop's layout checks itself
 * against the same file.
 */

const files = import.meta.glob<string>('../../../../fixtures/shell/v1/layout.json', { query: '?raw', import: 'default', eager: true });
type Entry = { command: string; option?: string };
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  key: string;
  saveMs: number;
  defaults: unknown;
  fields: unknown;
  limits: unknown;
  reads: { title: string; stored: string | null; layout: unknown }[];
  dockWidths: { kept: number; window: number; shown: number }[];
  bottomHeights: { kept: number; window: number; shown: number }[];
  layersDrags: { start: number; dy: number; height: number; fraction: number }[];
  ribbon: {
    quickAccessFixed: string[];
    quickAccess: { kept: string[]; exists: string[]; bar: string[] }[];
    splits: { entries: Entry[]; kept: string | null; current: number }[];
    startTabs: { kept: string; tabs: { id: string; contextual?: string }[]; tab: string }[];
  };
};

describe('the kept layout (fixtures/shell/v1/layout.json)', () => {
  it('is a v1 layout file: where it is kept, when it is written, its fields, defaults and limits', () => {
    expect([F.format, F.version]).toEqual(['kentos.layout', 1]);
    expect([LAYOUT_KEY, SAVE_DELAY_MS]).toEqual([F.key, F.saveMs]);
    expect(LAYOUT_DEFAULTS).toEqual(F.defaults);
    expect(LAYOUT_FIELDS).toEqual(F.fields);
    expect({ dockWidth: DOCK_WIDTH, layersFraction: LAYERS_FRACTION, bottomHeight: BOTTOM_HEIGHT }).toEqual(F.limits);
  });

  it('reads what was stored by each field’s rule, and drops what it no longer keeps', () => {
    for (const c of F.reads) expect(readLayout(c.stored), c.title).toEqual(c.layout);
  });

  it('shows the kept sizes within what the window allows, and drags the dock’s split within its limits', () => {
    for (const c of F.dockWidths) expect(dockWidthOn(c.kept, c.window), JSON.stringify(c)).toBe(c.shown);
    for (const c of F.bottomHeights) expect(bottomHeightOn(c.kept, c.window), JSON.stringify(c)).toBe(c.shown);
    for (const c of F.layersDrags) expect(draggedLayersFraction(c.start, c.dy, c.height), JSON.stringify(c)).toBe(c.fraction);
  });

  it('opens the kept ribbon tab, builds the quick access bar, and puts the last split choice on top', () => {
    expect(QUICK_ACCESS).toEqual(F.ribbon.quickAccessFixed);
    for (const c of F.ribbon.quickAccess) expect(quickAccessOf(c.kept, (id) => c.exists.includes(id)), JSON.stringify(c.kept)).toEqual(c.bar);
    for (const c of F.ribbon.splits) expect(splitCurrent(c.entries, c.kept ?? undefined), String(c.kept)).toBe(c.entries[c.current]);
    for (const c of F.ribbon.startTabs) expect(startTab(c.kept, c.tabs), `${c.kept} in ${c.tabs.map((t) => t.id).join(',')}`).toBe(c.tab);
  });
});
