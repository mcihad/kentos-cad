/**
 * The workbench's layout as the web keeps it (`ctx.ui`, localStorage
 * `kentos.ui.v1`; CLAUDE.md §4.4 “Yerleşim”): which panels are open, their
 * sizes, the dock's and the ribbon's states. What is kept, its defaults, how
 * a stored value is read (a value of the wrong type or out of its domain is
 * not taken; a field no longer kept, the toolbox's, is dropped:
 * docs/adr/0155), and the sizes as shown: a kept size is the user's wish,
 * shown within what the window allows. Apart from the DOM;
 * fixtures/shell/v1/layout.json holds it for the desktop (format in
 * fixtures/shell/README.md).
 */

export type Theme = 'dark' | 'light';
export type BottomTab = 'history' | 'coords' | 'points' | 'messages';
export type DockTab = 'layers' | 'processing' | 'blocks' | 'templates';
export type ProcessingTab = 'tools' | 'history';

export interface UiLayoutData {
  theme: Theme;
  rightVisible: boolean;
  dockWidth: number;
  /** Share of the right dock height given to the layer tree. */
  layersFraction: number;
  bottomExpanded: boolean;
  bottomHeight: number;
  bottomTab: BottomTab;
  /** Right dock's upper slot: the layer tree, the processing toolbox or the blocks (attributes below). */
  dockTab: DockTab;
  processingTab: ProcessingTab;
  /** Processing categories folded in the processing toolbox. */
  processingFolded: string[];
  /** Ribbon (Şerit): the open tab, folded to its tab row, and commands added to its quick access bar. */
  ribbonTab: string;
  ribbonCollapsed: boolean;
  ribbonQuickAccess: string[];
  /** The entry last chosen on each split button (Daire ▾: 3 nokta), by button key. */
  ribbonSplits: Record<string, string>;
}

/** Where the web keeps the layout. */
export const LAYOUT_KEY = 'kentos.ui.v1';

export const LAYOUT_DEFAULTS: UiLayoutData = {
  theme: 'dark',
  rightVisible: true,
  dockWidth: 312,
  layersFraction: 0.5,
  bottomExpanded: false,
  bottomHeight: 190,
  bottomTab: 'history',
  dockTab: 'layers',
  processingTab: 'tools',
  processingFolded: [],
  ribbonTab: 'home',
  ribbonCollapsed: false,
  ribbonQuickAccess: [],
  ribbonSplits: {},
};

/** The right dock's width (CSS px): dragged between `min` and the lesser of `max` and `maxShare` of the window's width. */
export const DOCK_WIDTH = { min: 240, max: 560, maxShare: 0.5, reset: 312 } as const;
/** The layer tree's share of the dock's height. */
export const LAYERS_FRACTION = { min: 0.15, max: 0.85, reset: 0.5 } as const;
/** The bottom panel's height (CSS px): at least `min`, at most `maxShare` of the window's height. */
export const BOTTOM_HEIGHT = { min: 96, maxShare: 0.6, reset: 190 } as const;

/** What a field's stored value must be to be taken. */
export type FieldRule =
  | { kind: 'enum'; values: readonly string[] }
  | { kind: 'boolean' }
  /** A finite number, brought within `min` and `max` when it is outside them. */
  | { kind: 'number'; min?: number; max?: number }
  | { kind: 'text' }
  /** A list: its texts are kept, anything else in it dropped. */
  | { kind: 'texts' }
  /** An object: its entries with a text value are kept. */
  | { kind: 'textMap' };

export const LAYOUT_FIELDS: { readonly [K in keyof UiLayoutData]: FieldRule } = {
  theme: { kind: 'enum', values: ['dark', 'light'] },
  rightVisible: { kind: 'boolean' },
  dockWidth: { kind: 'number', min: DOCK_WIDTH.min, max: DOCK_WIDTH.max },
  layersFraction: { kind: 'number', min: LAYERS_FRACTION.min, max: LAYERS_FRACTION.max },
  bottomExpanded: { kind: 'boolean' },
  bottomHeight: { kind: 'number', min: BOTTOM_HEIGHT.min },
  bottomTab: { kind: 'enum', values: ['history', 'coords', 'points', 'messages'] },
  dockTab: { kind: 'enum', values: ['layers', 'processing', 'blocks', 'templates'] },
  processingTab: { kind: 'enum', values: ['tools', 'history'] },
  processingFolded: { kind: 'texts' },
  ribbonTab: { kind: 'text' },
  ribbonCollapsed: { kind: 'boolean' },
  ribbonQuickAccess: { kind: 'texts' },
  ribbonSplits: { kind: 'textMap' },
};

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);

/** A stored value read by its field's rule; undefined when it cannot be taken. */
function readField(rule: FieldRule, v: unknown): unknown {
  switch (rule.kind) {
    case 'enum':
      return typeof v === 'string' && rule.values.includes(v) ? v : undefined;
    case 'boolean':
      return typeof v === 'boolean' ? v : undefined;
    case 'number':
      return typeof v === 'number' && Number.isFinite(v) ? Math.min(Math.max(v, rule.min ?? -Infinity), rule.max ?? Infinity) : undefined;
    case 'text':
      return typeof v === 'string' ? v : undefined;
    case 'texts':
      return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : undefined;
    case 'textMap':
      return isObject(v) ? Object.fromEntries(Object.entries(v).filter(([, x]) => typeof x === 'string')) : undefined;
  }
}

/**
 * The layout from what was stored (`null`: nothing): every field the store
 * holds and its rule takes; the default for the rest, and for everything
 * when the text is not a JSON object. Fields the layout does not know are
 * dropped (the next save leaves them out): the toolbox's among them, since
 * the web shows the ribbon only (docs/adr/0155).
 */
export function readLayout(text: string | null): UiLayoutData {
  let stored: unknown = null;
  try {
    stored = text === null ? null : JSON.parse(text);
  } catch {
    stored = null;
  }
  const saved = isObject(stored) ? stored : {};
  const out = { ...LAYOUT_DEFAULTS } as Record<string, unknown>;
  for (const [key, rule] of Object.entries(LAYOUT_FIELDS) as [keyof UiLayoutData, FieldRule][]) {
    const v = key in saved ? readField(rule, saved[key]) : undefined;
    if (v !== undefined) out[key] = v;
  }
  return out as unknown as UiLayoutData;
}

/** The dock's width as shown: the kept width within the limits the window allows now (the upper one wins in a very narrow window). */
export function dockWidthOn(kept: number, windowWidth: number): number {
  return Math.round(Math.min(Math.max(kept, DOCK_WIDTH.min), Math.min(DOCK_WIDTH.max, windowWidth * DOCK_WIDTH.maxShare)));
}

/** The bottom panel's height as shown: the kept height within the limits the window allows now. */
export function bottomHeightOn(kept: number, windowHeight: number): number {
  return Math.round(Math.min(Math.max(kept, BOTTOM_HEIGHT.min), windowHeight * BOTTOM_HEIGHT.maxShare));
}

/** The layer tree's share after its edge is dragged `dy` px down from `start` in a dock `height` px tall. */
export function draggedLayersFraction(start: number, dy: number, height: number): number {
  return Math.min(LAYERS_FRACTION.max, Math.max(LAYERS_FRACTION.min, start + dy / height));
}
