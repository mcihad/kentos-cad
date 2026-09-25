import type { AccentId, UiFontId } from './appearance';
import type { Workspace } from '../model/projectSettings';
import { Signal } from '../core/signal';
import type { LineType } from '../model/layers';

/** Drafting aids toggled from the status bar (F3/F7/F8/F10). */
export class DraftingSettings {
  readonly snap = new Signal(true);
  readonly grid = new Signal(true);
  readonly ortho = new Signal(false);
  readonly polar = new Signal(false);
  /** Object snap tracking: alignment lines from acquired snap points. */
  readonly tracking = new Signal(true);
  /** Current properties for new entities; null = katmana göre. */
  readonly color = new Signal<string | null>(null);
  readonly lineType = new Signal<LineType | null>(null);
  readonly lineWeight = new Signal<number | null>(null);
}

export type LogLevel = 'command' | 'info' | 'success' | 'warn' | 'error';

export interface LogEntry {
  id: number;
  time: Date;
  level: LogLevel;
  text: string;
}

export class MessageLog {
  readonly entries = new Signal<readonly LogEntry[]>([]);
  private seq = 0;

  push(level: LogLevel, text: string): void {
    const next = [...this.entries.value, { id: ++this.seq, time: new Date(), level, text }];
    this.entries.set(next.length > 500 ? next.slice(-500) : next);
  }

  command = (t: string) => this.push('command', t);
  info = (t: string) => this.push('info', t);
  success = (t: string) => this.push('success', t);
  warn = (t: string) => this.push('warn', t);
  error = (t: string) => this.push('error', t);

  clear(): void {
    this.entries.set([]);
  }
}

export type Theme = 'dark' | 'light';
export type BottomTab = 'history' | 'coords' | 'messages';
export type DockTab = 'layers' | 'processing';
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
  toolboxVisible: boolean;
  toolboxDocked: boolean;
  toolboxX: number;
  toolboxY: number;
  toolboxColumns: 2 | 3;
  /** Toolbox groups folded by the user (ToolGroup ids). */
  toolboxFolded: string[];
  /** Right dock content: layer tree and attributes, or the processing toolbox. */
  dockTab: DockTab;
  processingTab: ProcessingTab;
  /** Processing categories folded in the toolbox. */
  processingFolded: string[];
  /** Ribbon (Şerit): the open tab, folded to its tab row, and commands added to its quick access bar. */
  ribbonTab: string;
  ribbonCollapsed: boolean;
  ribbonQuickAccess: string[];
  /** The floating toolbox next to the ribbon (off by default: the ribbon holds every tool). */
  ribbonToolbox: boolean;
}

const DEFAULTS: UiLayoutData = {
  theme: 'dark',
  rightVisible: true,
  dockWidth: 312,
  layersFraction: 0.5,
  bottomExpanded: false,
  bottomHeight: 190,
  bottomTab: 'history',
  toolboxVisible: true,
  toolboxDocked: false,
  toolboxX: 12,
  toolboxY: 12,
  toolboxColumns: 3,
  toolboxFolded: [],
  dockTab: 'layers',
  processingTab: 'tools',
  processingFolded: [],
  ribbonTab: 'home',
  ribbonCollapsed: false,
  ribbonQuickAccess: [],
  ribbonToolbox: false,
};

export type Signals<T> = { readonly [K in keyof T]: Signal<T[K]> };

/**
 * One signal per field, persisted to localStorage under `key`. Storage may be
 * unavailable (private mode) or hold stale fields; unknown keys are dropped
 * and missing ones fall back to defaults.
 */
export function persistedSignals<T extends object>(key: string, defaults: T): Signals<T> {
  let saved: Partial<T> = {};
  try {
    saved = JSON.parse(localStorage.getItem(key) ?? '{}');
  } catch {
    /* ignore */
  }
  const state = Object.fromEntries(
    Object.entries(defaults).map(([k, v]) => [k, new Signal(k in saved ? (saved as Record<string, unknown>)[k] : v)]),
  ) as unknown as Signals<T>;
  let timer = 0;
  const save = () => {
    clearTimeout(timer);
    timer = window.setTimeout(() => {
      const snapshot = Object.fromEntries(Object.entries(state).map(([k, s]) => [k, (s as Signal<unknown>).value]));
      try {
        localStorage.setItem(key, JSON.stringify(snapshot));
      } catch {
        /* ignore */
      }
    }, 250);
  };
  for (const s of Object.values(state)) (s as Signal<unknown>).subscribe(save);
  return state;
}

/** Workspace layout (panel sizes, toolbox position, theme). */
export function createUiState(): Signals<UiLayoutData> {
  let legacyToolbox = false;
  try {
    const raw = localStorage.getItem('kentos.ui.v1');
    legacyToolbox = !!raw && !('toolboxFolded' in JSON.parse(raw));
  } catch {
    /* ignore */
  }
  const state = persistedSignals<UiLayoutData>('kentos.ui.v1', DEFAULTS);
  // Layouts saved before the titled toolbox groups used one or two columns;
  // the grouped toolbox is laid out for three.
  if (legacyToolbox) state.toolboxColumns.set(3);
  return state;
}
export type UiState = Signals<UiLayoutData>;

// ── Application preferences (Uygulama ayarları) ──────────────────────
// User-scoped, stored in this browser, valid for every project. Anything a
// colleague opening the same project must also see belongs in
// model/projectSettings.ts instead.

export type UiScale = 'small' | 'standard' | 'large' | 'xlarge' | 'xxlarge';
/** Workbench chrome: menu bar, toolbar and floating toolbox, or the tabbed ribbon (Şerit). */
export type ShellKind = 'classic' | 'ribbon';
export type CrosshairSize = 'small' | 'medium' | 'full';

export interface PreferencesData {
  /** EPSG code used for new projects. TUREF / TM36 by default. */
  defaultSrid: number;
  /** Work mode offered first for new projects (the project keeps its own, ProjectSettings.workspace). */
  defaultWorkspace: Workspace;
  /** Object snap and pick apertures in CSS px. */
  snapAperture: number;
  pickAperture: number;
  snapEndpoint: boolean;
  snapMidpoint: boolean;
  snapCenter: boolean;
  snapNode: boolean;
  snapIntersection: boolean;
  snapPerpendicular: boolean;
  snapNearest: boolean;
  snapTangent: boolean;
  /** Polar tracking step in degrees (F10 toggles tracking). */
  polarIncrement: number;
  crosshair: CrosshairSize;
  uiScale: UiScale;
  /** Accent colour of the interface and the drawing's selection (app/appearance.ts). */
  accent: AccentId;
  /** Interface typeface, bundled with the app (app/appearance.ts). */
  uiFont: UiFontId;
  rendererPreference: 'webgl2' | 'webgpu';
  /** Render at device pixel ratio; off trades sharpness for fill rate. */
  hiDpi: boolean;
  /** Typed values open beside the cursor while a command runs (dynamic input). */
  cursorInput: boolean;
  /** Resting the mouse on an object shows its kind, layer and measures. */
  hoverInfo: boolean;
  /**
   * Symbol sizes: "plot" = paper mm at the project's plot scale (they grow
   * and shrink with the map, as on the printed sheet); "screen" = mm on
   * the screen, the same size at every zoom (browsing).
   */
  symbolSize: 'plot' | 'screen';
  /**
   * Workbench chrome. Both are built from the same tool catalog and menu
   * model (app/menus.ts), so a new tool or command appears in either.
   */
  shell: ShellKind;
}

export const PREFERENCE_DEFAULTS: PreferencesData = {
  defaultSrid: 5256,
  defaultWorkspace: 'hybrid',
  snapAperture: 11,
  pickAperture: 5,
  snapEndpoint: true,
  snapMidpoint: true,
  snapCenter: true,
  snapNode: true,
  snapIntersection: true,
  snapPerpendicular: true,
  snapNearest: false,
  snapTangent: true,
  polarIncrement: 45,
  crosshair: 'medium',
  uiScale: 'standard',
  accent: 'navy',
  uiFont: 'jakarta',
  rendererPreference: 'webgl2',
  hiDpi: true,
  cursorInput: true,
  hoverInfo: true,
  symbolSize: 'plot',
  shell: 'classic',
};

export const createPreferences = () => persistedSignals<PreferencesData>('kentos.prefs.v1', PREFERENCE_DEFAULTS);
export type Preferences = Signals<PreferencesData>;

/** Plain snapshot, used by the settings dialog as an editable draft. */
export function snapshot<T extends object>(s: Signals<T>): T {
  return Object.fromEntries(Object.entries(s).map(([k, v]) => [k, (v as Signal<unknown>).value])) as T;
}
