import type { Capabilities } from '../../contracts/generated/sheet/Capabilities';
import type { GroundPoint } from '../../contracts/generated/sheet/GroundPoint';
import type { Guide } from '../../contracts/generated/sheet/Guide';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { RectUm } from '../../contracts/generated/sheet/RectUm';
import type { ResizeSnap } from '../../contracts/generated/sheet/ResizeSnap';
import type { SnapResult } from '../../contracts/generated/sheet/SnapResult';
import type { Workspace } from '../../contracts/generated/Workspace';
import type { BookText, SheetEngine } from '../../product/sheet/engine';
import type { SheetState } from '../../product/sheet/state';

/**
 * The paper's tools (docs/sheet/design.md §5, §11): what a press, a drag and
 * a release on the paper do. They never touch the DOM (the workspace gives
 * them points and modifiers) and decide nothing about geometry themselves:
 * what is under the pointer is the engine's `hitTest`, where a dragged
 * item lands is the engine's `SnapSession` (smart guides, distances and
 * equal spacing come back with it), the item a drag shows is the engine's
 * operation applied to a copy of the book, and the release applies that
 * same operation once, one undo step.
 */

/** A point on the paper, millimetres from its top left corner. */
export interface PaperPoint {
  readonly x: number;
  readonly y: number;
}

/** What a pointer event carries that a tool reads. */
export interface PointerMods {
  readonly shift: boolean;
  readonly ctrl: boolean;
  readonly alt: boolean;
}

/** What the paper shows over the sheet while a tool works (the painter draws it). */
export interface ToolOverlay {
  /** A box being dragged out (paper mm); `crossing` when right to left. */
  readonly marquee?: { readonly box: RectUm; readonly crossing: boolean };
  /** A drag's smart guides, distances and equal spacing (µm, the engine's). */
  readonly snap?: SnapResult;
  /** A resize's snapped edges and the items whose size it matches. */
  readonly resize?: ResizeSnap;
  /** A frame being drawn for a new item. */
  readonly frame?: RectUm;
  /** A guide being dragged from a ruler or along the paper (µm); `remove` when it would be dropped. */
  readonly guide?: { readonly axis: Guide['axis']; readonly at: number; readonly remove: boolean };
  /** The angle a rotation shows, degrees. */
  readonly angle?: { readonly at: PaperPoint; readonly degrees: number };
}

/** What a tool reaches: the engine, the sheet in front and its book, the view's scale, and the ways to show and apply. */
export interface PaperToolHost {
  readonly engine: SheetEngine;
  readonly state: SheetState;
  /** The book as it is (not the preview). */
  book(): BookText;
  /** The sheet in front. */
  sheet(): string;
  /** CSS pixels per millimetre at the view's zoom. */
  scale(): number;
  /** A book to show in place of the real one while a drag lasts; null shows the real one again. */
  preview(book: BookText | null): void;
  overlay(o: ToolOverlay | null): void;
  /** Applies operations as one undo step; false when refused (the host says why). */
  apply(ops: readonly Op[], label?: string): boolean;
  newId(): string;
  workspace(): Workspace;
  capabilities(): Capabilities;
  /** Where the drawing area looks: a new map's centre. */
  mapCenter(): GroundPoint;
  /** Says why something did not happen (the message log). */
  say(text: string): void;
  /** The pointer's look over the paper. */
  cursor(css: string): void;
  /** Gives the paper back to the select tool (an add tool's one item is placed). */
  done(): void;
}

export interface PaperTool {
  /** A press on the paper; true when the tool takes the drag that follows. */
  down(p: PaperPoint, m: PointerMods): boolean;
  move(p: PaperPoint, m: PointerMods): void;
  up(p: PaperPoint, m: PointerMods): void;
  /** The pointer over the paper with no button down. */
  hover?(p: PaperPoint | null): void;
  /** Esc, a lost pointer, another tool: the drag is dropped, nothing is applied. */
  cancel(): void;
}

/** Millimetres as the engine's micrometres. */
export const toUm = (mm: number): number => Math.round(mm * 1000);

/** A box between two paper points, µm. */
export function boxUm(a: PaperPoint, b: PaperPoint): RectUm {
  const l = Math.min(a.x, b.x);
  const t = Math.min(a.y, b.y);
  return { left: toUm(l), top: toUm(t), width: toUm(Math.max(a.x, b.x)) - toUm(l), height: toUm(Math.max(a.y, b.y)) - toUm(t) };
}

/** The engine's snapping tolerance (screen pixels, `EngineInfo.tolerancePx`) in µm at the view's zoom. */
export const toleranceUm = (host: PaperToolHost): number => (host.engine.info.tolerancePx / Math.max(1e-6, host.scale())) * 1000;

/** How far a press must move (CSS px) before it is a drag and not a click. */
export const DRAG_PX = 3;
