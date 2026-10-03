import type { AccentId, CornersId, ShadowsId, ThemeId, UiFontId } from './appearance';
import type { DrawingFont, DrawingUnit, Workspace } from '../model/projectSettings';
import { Signal } from '../core/signal';
import { settingDefault } from '../core/settings/schema';
import type { LineType } from '../model/layers';
import { LAYOUT_DEFAULTS, LAYOUT_KEY, readLayout, type UiLayoutData } from './layoutPlan';
import type { SettingsStore } from './settings/store';
import { NO_LOCKS, type LockAsk, type LockState } from '../tools/locks';

const sessionDefault = (key: string) => settingDefault(key) as boolean;

/** The overlap control's modes (`drafting.overlap`, docs/adr/0162 §1): Serbest, Kendi katmanında önle, Seçili katmanlarda önle. */
export type OverlapMode = 'allow' | 'layer' | 'layers';

/**
 * Drafting aids toggled from the status bar (F3/F7/F8/F10): the typed
 * schema's session settings (`drafting.*`, docs/adr/0023), so every session
 * starts from the schema's defaults.
 */
export class DraftingSettings {
  readonly snap = new Signal(sessionDefault('drafting.snap'));
  readonly grid = new Signal(sessionDefault('drafting.grid'));
  readonly ortho = new Signal(sessionDefault('drafting.ortho'));
  readonly polar = new Signal(sessionDefault('drafting.polar'));
  /** Object snap tracking: alignment lines from acquired snap points. */
  readonly tracking = new Signal(sessionDefault('drafting.tracking'));
  /** Topological editing (docs/adr/0160): an edit puts its neighbours' shared corners and edges right with it. */
  readonly topology = new Signal(sessionDefault('drafting.topology'));
  /** Points count as shared corners too (Noktalar da). */
  readonly topologyPoints = new Signal(sessionDefault('drafting.topologyPoints'));
  /** The overlap control (docs/adr/0162 §1): a new area drawn by its outline loses what overlaps its neighbours. */
  readonly overlap = new Signal(settingDefault('drafting.overlap') as OverlapMode);
  /** The mode the cell's click turns on again: the last that avoided overlap. */
  readonly overlapLast = new Signal<Exclude<OverlapMode, 'allow'>>('layer');
  /** Seçili katmanlarda önle's layers, by id: the session's, not a setting (settings hold no lists). */
  readonly overlapLayers = new Signal<ReadonlySet<string>>(new Set());
  /** The digitizing locks (docs/adr/0166): what holds the next point; a new command starts with none. */
  readonly locks = new Signal<LockState>(NO_LOCKS);
  /** What the value card asks for after a lock was chosen from the menu (Uzunluk…, Açı…, Sapma…). */
  readonly lockAsk = new Signal<LockAsk | null>(null);
  /** The value card is open beside the cursor: its chips say the locks, so the drawing's lock tag gives way. */
  readonly valueCard = new Signal(false);
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

/** The log keeps this many lines; the oldest go first (fixtures/shell/v1/log.json). */
export const LOG_LIMIT = 500;

export class MessageLog {
  readonly entries = new Signal<readonly LogEntry[]>([]);
  /** Ids only grow, across Geçmişi temizle too (the Uyarılar badge counts from them, ui/bottom/warnings.ts). */
  private seq = 0;

  push(level: LogLevel, text: string): void {
    const next = [...this.entries.value, { id: ++this.seq, time: new Date(), level, text }];
    this.entries.set(next.length > LOG_LIMIT ? next.slice(-LOG_LIMIT) : next);
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

export type { BottomTab, DockTab, ProcessingTab, Theme, UiLayoutData } from './layoutPlan';

export type Signals<T> = { readonly [K in keyof T]: Signal<T[K]> };

/** A store is written this long after its last change (ms): one write for a drag, not one per step. */
export const SAVE_DELAY_MS = 250;

/** What was stored under `key`, or null (nothing, or storage refused: private mode). */
function stored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/**
 * One signal per field, persisted to localStorage under `key`, every field
 * written SAVE_DELAY_MS after the last change. Storage may be unavailable
 * (private mode) or hold stale fields. `read` turns what was stored into the
 * values; by default a field stored is taken as it is and a missing one is
 * its default. Fields the defaults do not name are dropped: the next write
 * leaves them out.
 */
export function persistedSignals<T extends object>(key: string, defaults: T, read?: (text: string | null) => T): Signals<T> {
  let values: T;
  if (read) values = read(stored(key));
  else {
    let saved: Partial<T> = {};
    try {
      saved = JSON.parse(stored(key) ?? '{}');
    } catch {
      /* ignore */
    }
    values = Object.fromEntries(Object.entries(defaults).map(([k, v]) => [k, k in saved ? (saved as Record<string, unknown>)[k] : v])) as T;
  }
  const state = Object.fromEntries(Object.entries(values).map(([k, v]) => [k, new Signal(v)])) as unknown as Signals<T>;
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
    }, SAVE_DELAY_MS);
  };
  for (const s of Object.values(state)) (s as Signal<unknown>).subscribe(save);
  return state;
}

/** Workspace layout (panels, sizes, dock and ribbon states): app/layoutPlan.ts reads it. */
export function createUiState(): Signals<UiLayoutData> {
  return persistedSignals<UiLayoutData>(LAYOUT_KEY, LAYOUT_DEFAULTS, readLayout);
}
export type UiState = Signals<UiLayoutData>;

// ── Application preferences (Uygulama ayarları) ──────────────────────
// Valid for every project. Each is a typed setting (docs/adr/0023): the
// schema gives its type, domain, default and scope (the user's preference, or
// this device's for the drawing engine); the settings service keeps and
// resolves it (app/settings/store.ts, localStorage kentos.settings.v1).
// Anything a colleague opening the same project must also see belongs in
// model/projectSettings.ts instead.

export type CrosshairSize = 'small' | 'medium' | 'full';

export interface PreferencesData {
  /** EPSG code used for new projects. TUREF / TM36 by default. */
  defaultSrid: number;
  /** Work mode offered first for new projects (the project keeps its own, ProjectSettings.workspace). */
  defaultWorkspace: Workspace;
  /** Drawing typeface for new projects (the project keeps its own, ProjectSettings.drawingFont). */
  defaultDrawingFont: DrawingFont;
  /** The unit a new local CAD project is drawn in (the project keeps its own, ProjectSettings.drawingUnit; docs/adr/0165 §2). */
  defaultDrawingUnit: DrawingUnit;
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
  /** The snap additions (docs/adr/0163 §1): a closed area's centroid, an acquired end's extension, the parallel to an acquired edge, Karelaj. */
  snapCentroid: boolean;
  snapExtension: boolean;
  snapParallel: boolean;
  snapGrid: boolean;
  /** Karelaj's spacings, east (Y) and north (X), metres. */
  snapGridEast: number;
  snapGridNorth: number;
  /** Snap to the object being drawn too (§3). */
  snapSelf: boolean;
  /** Snap only between these screen scales, 1:N (0: no limit; §5). */
  snapScaleMin: number;
  snapScaleMax: number;
  /** Polar tracking step in degrees (F10 toggles tracking). */
  polarIncrement: number;
  crosshair: CrosshairSize;
  /** The interface's colours (app/appearance.ts); the layout kept it before (docs/adr/0126). */
  theme: ThemeId;
  /** The interface's text size in pixels; every size of the chrome scales with it. */
  textSize: number;
  /** Every radius together (docs/adr/0127). */
  corners: CornersId;
  /** The shadow of what floats: menus, pop-ups, dialogs. */
  shadows: ShadowsId;
  /** Accent colour of the interface and the drawing's selection (app/appearance.ts). */
  accent: AccentId;
  /** Interface typeface, bundled with the app (app/appearance.ts). */
  uiFont: UiFontId;
  /** The drawing engine this device starts with (`?renderer=` overrides it for the session). */
  rendererPreference: 'webgl2' | 'webgpu';
  /**
   * Multisampling of the drawing: 1 (off), 2, 4, 8 or 16 samples per pixel;
   * the count in use, which this device's GPU may hold below the one asked
   * for (the settings service keeps that). Changes apply at once.
   */
  msaa: number;
  /** Draw at the screen's full resolution (Retina, 4K); off: one pixel per CSS pixel. */
  hiDpi: boolean;
  /** Typed values open beside the cursor while a command runs (dynamic input). */
  cursorInput: boolean;
  /** The strip over the drawing while a command runs: its step and options as buttons (ui/shell/CommandBar.ts). */
  commandBar: boolean;
  /** Resting the mouse on an object shows its kind, layer and measures. */
  hoverInfo: boolean;
  /**
   * Symbol sizes: "plot" = paper mm at the project's plot scale (they grow
   * and shrink with the map, as on the printed sheet); "screen" = mm on
   * the screen, the same size at every zoom (browsing).
   */
  symbolSize: 'plot' | 'screen';
  /**
   * Line weights shown (AutoCAD's LWT): off, every layer line is drawn one pixel thin, for precise work
   * among thick boundaries. Symbols from the style library keep their own widths.
   */
  lineWeights: boolean;
  /** The start screen (Başlangıç: new, open, cloud, recent files) shows when the app opens. */
  startScreen: boolean;
}

/** The typed setting behind each preference (settingsSchema.json). */
export const PREF_KEYS = {
  defaultSrid: 'newProjects.srid',
  defaultWorkspace: 'newProjects.workspace',
  defaultDrawingFont: 'newProjects.drawingFont',
  defaultDrawingUnit: 'newProjects.drawingUnit',
  snapAperture: 'drafting.snapAperture',
  pickAperture: 'drafting.pickAperture',
  snapEndpoint: 'snap.endpoint',
  snapMidpoint: 'snap.midpoint',
  snapCenter: 'snap.center',
  snapNode: 'snap.node',
  snapIntersection: 'snap.intersection',
  snapPerpendicular: 'snap.perpendicular',
  snapNearest: 'snap.nearest',
  snapTangent: 'snap.tangent',
  snapCentroid: 'snap.centroid',
  snapExtension: 'snap.extension',
  snapParallel: 'snap.parallel',
  snapGrid: 'snap.grid',
  snapGridEast: 'snap.gridEast',
  snapGridNorth: 'snap.gridNorth',
  snapSelf: 'snap.self',
  snapScaleMin: 'snap.scaleMin',
  snapScaleMax: 'snap.scaleMax',
  polarIncrement: 'drafting.polarIncrement',
  crosshair: 'appearance.crosshair',
  theme: 'appearance.theme',
  textSize: 'appearance.textSize',
  corners: 'appearance.corners',
  shadows: 'appearance.shadows',
  accent: 'appearance.accent',
  uiFont: 'appearance.uiFont',
  rendererPreference: 'graphics.backend',
  msaa: 'graphics.msaa',
  hiDpi: 'graphics.hiDpi',
  cursorInput: 'drafting.cursorInput',
  commandBar: 'drafting.commandBar',
  hoverInfo: 'drafting.hoverInfo',
  symbolSize: 'graphics.symbolSize',
  lineWeights: 'graphics.lineWeights',
  startScreen: 'appearance.startScreen',
} as const satisfies Record<keyof PreferencesData, string>;

/** The defaults, from the settings schema. */
export const PREFERENCE_DEFAULTS = Object.fromEntries(Object.entries(PREF_KEYS).map(([field, key]) => [field, settingDefault(key)])) as unknown as PreferencesData;

/**
 * One signal per preference, holding the value in use: the effective one, as
 * a device limit or the organisation's policy may keep it from the one asked
 * for. Setting a signal writes the preference into the settings service; a
 * value the rules refuse is not taken and the signal goes back to the value
 * in use. An import, a reset or a new constraint updates the signals.
 */
export function createPreferences(store: SettingsStore): Preferences {
  const fields = Object.entries(PREF_KEYS) as [keyof PreferencesData, string][];
  const signals = new Map<keyof PreferencesData, Signal<unknown>>();
  let syncing = false;
  const sync = () => {
    syncing = true;
    try {
      for (const [field, key] of fields) signals.get(field)!.set(store.effective(key));
    } finally {
      syncing = false;
    }
  };
  for (const [field, key] of fields) {
    const s = new Signal<unknown>(store.effective(key));
    s.subscribe((value) => {
      if (syncing) return;
      store.choose({ [key]: value });
      sync();
    });
    signals.set(field, s);
  }
  store.revision.subscribe(sync);
  return Object.fromEntries(signals) as unknown as Preferences;
}
export type Preferences = Signals<PreferencesData>;

/** Plain snapshot, used by the settings dialog as an editable draft. */
export function snapshot<T extends object>(s: Signals<T>): T {
  return Object.fromEntries(Object.entries(s).map(([k, v]) => [k, (v as Signal<unknown>).value])) as T;
}
