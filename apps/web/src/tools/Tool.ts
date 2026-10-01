import type { AppContext } from '../app/context';
import type { ReadonlySignal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import type { TrackHit } from '../viewport/objectTracking';
import type { SnapHit } from '../viewport/picking';

export interface ToolPointer {
  /** Absolute world position after snapping. */
  world: Vec2;
  /** Absolute world position before snapping. */
  raw: Vec2;
  /** CSS px inside the viewport. */
  screen: Vec2;
  snap: SnapHit | null;
  /** Object tracking lock (alignment with acquired points), when no snap applies. */
  track: TrackHit | null;
  button: number;
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
}

export type ToolCursor = 'pick' | 'cross' | 'grab';

/** What was held when a tool was confirmed from the keyboard (Shift+Enter). */
export interface ConfirmMods {
  readonly shift?: boolean;
}

/**
 * Interactive tool contract. The viewport feeds pointer events, the command
 * line feeds typed input; the tool never touches the DOM directly.
 */
/** One value an option offers in its menu (Yazı's Hiza, docs/adr/0145 §6). */
export interface OptionChoice {
  label: string;
  /** What typing it after the option's key gives (“sağ üst”): the command history says it. */
  typed: string;
  icon?: string;
  checked: boolean;
}

export interface Tool {
  readonly id: string;
  /** Text the command line shows while the tool waits for input. */
  readonly prompt: ReadonlySignal<string>;
  readonly cursor: ToolCursor;
  /** Whether object snaps apply while this tool runs. */
  readonly snaps: boolean;
  activate?(): void;
  deactivate?(): void;
  pointerDown?(p: ToolPointer): void;
  pointerMove?(p: ToolPointer): void;
  pointerUp?(p: ToolPointer): void;
  /** Typed coordinate, number or option. Return false when not understood. */
  input?(text: string): boolean;
  /**
   * The values an option chooses between, for its button and the right-button menu to offer as a menu (Yazı's
   * Hiza, docs/adr/0145 §6); none: the button sends the key.
   */
  optionChoices?(key: string): readonly OptionChoice[] | null;
  /** One of `optionChoices(key)` chosen, as typing the key and then `typed`; false when this step takes none. */
  chooseOption?(key: string, typed: string): boolean;
  /** Enter, Space or right click; Shift+Enter says `shift` (Çitle seç adds to the selection then, docs/adr/0141). */
  confirm?(mods?: ConfirmMods): void;
  /**
   * Ctrl+Z while the tool runs: takes back its newest step (a point of the
   * draft) and returns true; false when nothing is pending, and the
   * drawing is undone instead (docs/adr/0018).
   */
  undoStep?(): boolean;
  /**
   * Esc. Return true when the tool handled it internally (e.g. dropped a
   * hot grip) and should stay active; otherwise the manager leaves the tool.
   */
  cancel?(): boolean;
  /** Reference point for perpendicular snaps (usually the last picked point). */
  snapFrom?(): Vec2 | null;
  /** Grip currently being edited, drawn as the hot grip. */
  activeGrip?(): { id: number; index: number } | null;
  /**
   * A point computed elsewhere (the point calculator) given as if it had
   * been clicked. False when the current stage does not take a point.
   */
  acceptPoint?(p: Vec2): boolean;
  /** Screen-space preview drawn on the overlay canvas. */
  draw?(g: CanvasRenderingContext2D, view: ViewTransform): void;
  /**
   * Points the running command has taken so far. The interaction traces
   * (fixtures/interaction, docs/adr/0018) read it; desktop tools report the same.
   */
  readonly pointCount?: number;
}

/**
 * Whether typed text goes to the running tool (`input`) rather than naming a
 * command: any tool but selection, and selection while one of its grips is
 * being moved, whose prompt asks for the new place (“koordinat yazın”).
 * The command line and the field beside the cursor ask the same.
 */
export function takesTypedInput(activeId: string, tool: Pick<Tool, 'activeGrip'>): boolean {
  return activeId !== 'select' || !!tool.activeGrip?.();
}

export type ToolGroup = 'select' | 'draw' | 'annotate' | 'block' | 'transform' | 'modify' | 'area' | 'map';

/** Short on purpose: these are the toolbox section headings. */
export const TOOL_GROUP_LABEL: Record<ToolGroup, string> = {
  select: 'Seçim',
  draw: 'Çizim',
  annotate: 'Açıklama',
  block: 'Blok',
  transform: 'Dönüştür',
  modify: 'Düzenle',
  area: 'Alan',
  map: 'Harita',
};

/**
 * Sub-headings inside a group, in display order: the separators of the
 * classic menus and the panels of the ribbon. A tool names its section in
 * the catalog; a group without sections (or a tool without one) is shown
 * under the group's own name, so a new tool always appears somewhere.
 */
export const TOOL_SECTIONS = {
  draw: { line: 'Çizgi', curve: 'Eğri', shape: 'Şekil', construction: 'Yardımcı', point: 'Nokta' },
  transform: { move: 'Dönüştür', array: 'Dizi' },
  modify: { edge: 'Kenar', corner: 'Köşe', object: 'Nesne', elevation: 'Kot' },
  area: { create: 'Oluştur ve çevir', boolean: 'Birleştir ve böl' },
  map: { parcel: 'Parsel', field: 'Arazi', measure: 'Ölçme' },
} as const satisfies Partial<Record<ToolGroup, Record<string, string>>>;

type Sectioned = typeof TOOL_SECTIONS;
export type ToolSection = { [G in keyof Sectioned]: keyof Sectioned[G] }[keyof Sectioned];

/** One way of starting a tool (AutoCAD's Daire ▾ list): the option typed right after it starts. */
export interface ToolMethod {
  readonly label: string;
  /** The prompt option sent after the tool starts (`2N`); none for the tool's own first step. */
  readonly option?: string;
  readonly description?: string;
  /** Typed names that start the tool with this method (`DOR`: Ölçülendirme as Koordinat; docs/adr/0147 §7). */
  readonly aliases?: readonly string[];
}

export interface ToolDescriptor {
  id: string;
  label: string;
  icon: string;
  group: ToolGroup;
  /** Sub-heading inside the group (TOOL_SECTIONS); must be one of the group's own. */
  section?: ToolSection;
  shortcut?: string;
  aliases?: readonly string[];
  description: string;
  /** Mouse-first "how to use" steps, shown in the toolbox tooltip. */
  steps?: readonly string[];
  /**
   * Ribbon presentation (app/ribbon.ts): a main tool of its panel, drawn
   * large; the others are small, three to a column. At most four a panel.
   */
  primary?: boolean;
  /** Ribbon: tools of one family share a split button showing the one last chosen (Dikdörtgen ▾). */
  family?: string;
  /** Ribbon: the ways to start the tool, listed under its split button (Daire ▾: 2 nokta, 3 nokta …). */
  methods?: readonly ToolMethod[];
  /** Ribbon: seldom used; listed under the panel's ▾ instead of on the panel. */
  rare?: boolean;
  /**
   * The product command (docs/adr/0013, 0022) the tool's confirm runs, by
   * catalog id: the typed operation the interaction ends in. The inventory
   * records it; only tools that write through one name it.
   */
  productCommand?: string;
  /** False for tools whose behaviour is not built yet. */
  ready: boolean;
  create(ctx: AppContext): Tool;
}
