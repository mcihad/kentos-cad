import type { Item } from '../../contracts/generated/sheet/Item';
import type { Paper } from '../../contracts/generated/sheet/Paper';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';

/**
 * The sheet layouts as the interface shows them (docs/sheet/design.md §11): the
 * open project's sheets, their paper and their items, in the words the tab
 * strip, the panels, the inspector and the workspace use. The engine's book
 * (kentos.sheet/1, its types written by kentos-sheet into
 * contracts/generated/sheet/) is turned into them by its adapter
 * (adapter.ts); each view keeps the engine's own record beside it
 * (`source`), which the inspector reads a kind's properties from and builds
 * its patches on. Lengths here are millimetres of paper (the engine keeps
 * micrometres), a position is measured from the paper's top left corner
 * downwards (design §2) and the interface names it Sol, Üst, Genişlik,
 * Yükseklik: X and Y are ground coordinates in KentOS.
 */

/** A box on the paper, in millimetres from its top left corner. */
export interface RectMm {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

export interface MarginsMm {
  readonly left: number;
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
}

export type Orientation = 'portrait' | 'landscape';

/** A sheet's paper: its name (A3, A1, Özel), its way round, its size and its margins. */
export interface PaperView {
  /** The engine's paper id (`a3`, `custom`). */
  readonly paper: Paper;
  readonly name: string;
  readonly orientation: Orientation;
  readonly widthMm: number;
  readonly heightMm: number;
  readonly margins: MarginsMm;
}

/** What an item is, as the interface names and draws it (its icon, its label, its section in the inspector). */
export type ItemKindView = 'map' | 'text' | 'scaleBar' | 'northArrow' | 'legend' | 'picture' | 'shape' | 'line' | 'table' | 'coordinateList' | 'titleBlock' | 'border' | 'group';

/** Which distance an item keeps when the paper changes (design §3.2), as the constraint editor shows it. */
export type HAnchor = 'left' | 'right' | 'leftRight' | 'center' | 'scale';
export type VAnchor = 'top' | 'bottom' | 'topBottom' | 'center' | 'scale';
/** The box the distances are kept to. */
export type AnchorBox = 'page' | 'margins' | 'group';

export interface AnchorsView {
  readonly h: HAnchor;
  readonly v: VAnchor;
  readonly box: AnchorBox;
}

export interface ItemView {
  /** Its lasting key: selection, the tree and the inspector name it by this. */
  readonly id: string;
  /** Unique on its sheet; commands and scripts name it by this. */
  readonly name: string;
  readonly kind: ItemKindView;
  /** Its frame before rotation. */
  readonly frame: RectMm;
  /** Degrees, clockwise, about the frame's centre. */
  readonly rotation: number;
  readonly locked: boolean;
  readonly hidden: boolean;
  readonly printable: boolean;
  /** 0–100. */
  readonly opacity: number;
  readonly anchors: AnchorsView;
  /** The group item it belongs to. */
  readonly group: string | null;
  /** Properties bound to data (ƒ), by the property's path ("frame.left", "text.content"). */
  readonly bindings: Readonly<Record<string, string>>;
  /** It belongs to the sheet's master page: drawn under the sheet's own, locked, not selected. */
  readonly master?: boolean;
  /**
   * It is a tool of another work mode (design §11a, the zone-marked frame is CAD's, the attribute table
   * GIS's): it stays, is drawn and edited; only new ones are not added in this mode. The engine's note
   * saying so (`itemNote`), for the inspector.
   */
  readonly note?: string;
  /** One line under its name in the tree and the inspector (“1/1000”, “3 satır”). */
  readonly detail?: string;
  /** The engine's item. */
  readonly source: Item;
}

export interface SheetView {
  readonly id: string;
  readonly name: string;
  readonly paper: PaperView;
  /** Drawing order: the first is at the bottom. A master page's items come first. */
  readonly items: readonly ItemView[];
  /** The master page it is drawn over, by name. */
  readonly master?: string;
  /** The template it was made from, and whether that template has a newer revision (design §12). */
  readonly template?: { readonly name: string; readonly newer: boolean };
  /** The layout in force for its paper (design §3.2a), by name; none: the base layout. */
  readonly layout?: string;
  /** The engine's sheet. */
  readonly source: Sheet;
}

export interface BookView {
  readonly sheets: readonly SheetView[];
  /** The engine's book; null before one is read (or while a stored one waits for the engine). */
  readonly source: SheetBook | null;
}

export const EMPTY_BOOK: BookView = { sheets: [], source: null };

/** The items a sheet's own panels list and its selection may hold: not its master page's. */
export const ownItems = (sheet: SheetView): ItemView[] => sheet.items.filter((i) => !i.master);

/** An item's place on its sheet, by id. */
export const itemById = (sheet: SheetView | null, id: string): ItemView | undefined => sheet?.items.find((i) => i.id === id);
