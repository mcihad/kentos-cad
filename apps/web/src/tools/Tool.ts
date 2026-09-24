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

/**
 * Interactive tool contract. The viewport feeds pointer events, the command
 * line feeds typed input; the tool never touches the DOM directly.
 */
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
  /** Enter or right click. */
  confirm?(): void;
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
}

export type ToolGroup = 'select' | 'draw' | 'annotate' | 'transform' | 'modify' | 'area' | 'map';

/** Short on purpose: these are the toolbox section headings. */
export const TOOL_GROUP_LABEL: Record<ToolGroup, string> = {
  select: 'Seçim',
  draw: 'Çizim',
  annotate: 'Açıklama',
  transform: 'Dönüştür',
  modify: 'Düzenle',
  area: 'Alan',
  map: 'Harita',
};

export interface ToolDescriptor {
  id: string;
  label: string;
  icon: string;
  group: ToolGroup;
  shortcut?: string;
  aliases?: readonly string[];
  description: string;
  /** Mouse-first "how to use" steps, shown in the toolbox tooltip. */
  steps?: readonly string[];
  /** False for tools whose behaviour is not built yet. */
  ready: boolean;
  create(ctx: AppContext): Tool;
}
