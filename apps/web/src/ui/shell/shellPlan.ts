import type { ShellKind } from '../../app/state';
import type { ToolGroup } from '../../tools/Tool';

/**
 * The classic shell's rules apart from its DOM (DESIGN.md §7.1, §7.3, §7.4):
 * what each fold step of the menu bar and the toolbar does, the toolbox's
 * place, its columns and when it shows, and the words of these bars.
 * MenuBar.ts, Toolbar.ts and Toolbox.ts apply them; whether a step fits is
 * measured there. fixtures/shell/v1/shell.json holds them for the desktop
 * (format in fixtures/shell/README.md).
 */

/** One step of the menu bar making room: what shows, and the menus' side padding (CSS px). */
export interface MenubarFold {
  /** The “KentOS” word beside the mark. */
  readonly brandWord: boolean;
  /** The coordinate system's name on its button (the button and its tip stay). */
  readonly crsName: boolean;
  readonly menuPadding: number;
}

/**
 * The menu bar's steps, tried in order while the menus overflow: the word,
 * then the CRS's name, then tighter menus. The document's name shrinks
 * before any of them by itself.
 */
export const MENUBAR_FOLD: readonly MenubarFold[] = [
  { brandWord: true, crsName: true, menuPadding: 9 },
  { brandWord: false, crsName: true, menuPadding: 9 },
  { brandWord: false, crsName: false, menuPadding: 9 },
  { brandWord: false, crsName: false, menuPadding: 6 },
];

/** The toolbar's groups, in order: commands, or the current-property fields (`fields`) and the scale (`scale`). The spacer goes before the scale. */
export const TOOLBAR_GROUPS: readonly { readonly label: string; readonly commands?: readonly string[]; readonly fields?: true; readonly scale?: true }[] = [
  { label: 'Dosya', commands: ['file.new', 'file.open', 'file.save'] },
  { label: 'Geçmiş', commands: ['edit.undo', 'edit.redo'] },
  { label: 'Görünüm', commands: ['view.zoomExtents', 'tool.zoomWindow', 'view.zoomSelection', 'view.zoomIn', 'view.zoomOut', 'tool.pan'] },
  { label: 'Geçerli özellikler', fields: true },
  { label: 'Çizim ölçeği', scale: true },
  { label: 'Paneller', commands: ['view.toolbox', 'view.bottomPanel', 'view.rightPanel'] },
];

/** Görünüm commands that fold under ⋯: the wheel and the middle button do them too. */
export const VIEW_MORE: readonly string[] = ['view.zoomIn', 'view.zoomOut', 'tool.pan'];

/** The fields' widths in CSS px at the standard type scale, full then narrow. The scale field keeps its width: “1:1000” shows whole. */
export const TOOLBAR_WIDTHS = { layer: [196, 150], color: [150, 124], lineType: [150, 124], weight: [160, 132] } as const;

/** One step of the toolbar making room. */
export interface ToolbarFold {
  /** The layer field at its narrow width. */
  readonly layerNarrow: boolean;
  /** Renk, Tip and Kalınlık at their narrow widths. */
  readonly propsNarrow: boolean;
  /** Yakınlaştır, Uzaklaştır and Kaydır under the Görünüm group's ⋯. */
  readonly viewMore: boolean;
  /** Renk, Tip and Kalınlık folded into the one “Özellikler” field. */
  readonly propsFolded: boolean;
}

/**
 * The toolbar's steps (DESIGN.md §7.3), tried in order until it fits:
 * 1. the fields narrow; 2. the three Görünüm commands go under ⋯;
 * 3. the three property fields fold into one, the layer field gets its
 * width back; 4. the layer field narrows again.
 */
export const TOOLBAR_FOLD: readonly ToolbarFold[] = [
  { layerNarrow: false, propsNarrow: false, viewMore: false, propsFolded: false },
  { layerNarrow: true, propsNarrow: true, viewMore: false, propsFolded: false },
  { layerNarrow: true, propsNarrow: true, viewMore: true, propsFolded: false },
  { layerNarrow: false, propsNarrow: true, viewMore: true, propsFolded: true },
  { layerNarrow: true, propsNarrow: true, viewMore: true, propsFolded: true },
];

/** The toolbox's groups, in order. */
export const TOOLBOX_GROUPS: readonly ToolGroup[] = ['select', 'draw', 'annotate', 'transform', 'modify', 'area', 'map'];

/** The floating toolbox: its margin inside the drawing, how near an edge it snaps, the widest it grows, and where a drag from the dock puts it (the cursor minus this). */
export const TOOLBOX = { margin: 8, snap: 14, maxColumns: 6, undock: { x: 20, y: 10 } } as const;

/** The stored column choice as the toolbox reads it: three, or anything else two. */
export const toolboxColumns = (stored: number): 2 | 3 => (stored === 3 ? 3 : 2);

/**
 * The columns the toolbox takes: the chosen count, widened one at a time
 * until every tool fits the height (`fits`), at most TOOLBOX.maxColumns; it
 * never scrolls.
 */
export function fitColumns(base: number, fits: (columns: number) => boolean): number {
  let columns = base;
  while (columns < TOOLBOX.maxColumns && !fits(columns)) columns++;
  return columns;
}

/**
 * Where the floating toolbox goes for a wanted place: inside the drawing
 * area (`hostW` × `hostH`) with the margin, and onto the margin when it comes
 * within the snap distance of an edge.
 */
export function toolboxPlace(x: number, y: number, w: number, h: number, hostW: number, hostH: number): { x: number; y: number } {
  const { margin, snap } = TOOLBOX;
  const maxX = Math.max(margin, hostW - w - margin);
  const maxY = Math.max(margin, hostH - h - margin);
  let nx = Math.min(Math.max(x, margin), maxX);
  let ny = Math.min(Math.max(y, margin), maxY);
  if (nx - margin < snap) nx = margin;
  if (maxX - nx < snap) nx = maxX;
  if (ny - margin < snap) ny = margin;
  if (maxY - ny < snap) ny = maxY;
  return { x: nx, y: ny };
}

/** Where a docked toolbox dragged by its grip lands, under the cursor (`x`, `y` in the drawing area's coordinates). */
export const undockAt = (x: number, y: number): { x: number; y: number } => ({ x: x - TOOLBOX.undock.x, y: y - TOOLBOX.undock.y });

/** Whether the toolbox shows: with the classic shell unless hidden; next to the ribbon (which holds every tool) only when asked for. */
export const toolboxShown = (shell: ShellKind, visible: boolean, ribbonToolbox: boolean): boolean => (shell === 'ribbon' ? ribbonToolbox : visible);

/** The bars' words. */
export const SHELL_TEXTS = {
  menubar: {
    label: 'Ana menü',
    dirty: 'Kaydedilmemiş değişiklikler var',
    crs: 'Koordinat sistemi',
    crsTip: (srid: number) => `EPSG:${srid}. Değiştirmek için tıklayın.`,
  },
  toolbar: {
    label: 'Araç çubuğu',
    more: 'Diğer görünüm komutları',
    moreTip: 'Yakınlaştır, Uzaklaştır ve Kaydır: pencere daralınca buraya girer.',
  },
  toolbox: {
    label: 'Çizim araçları',
    grip: 'Taşımak için sürükleyin',
    columns: 'Sütun sayısını değiştir',
    twoColumns: 'İki sütun',
    threeColumns: 'Üç sütun',
    dock: 'Kenara sabitle',
    undock: 'Serbest bırak',
    fold: (group: string) => `${group} grubunu katla`,
    unfold: (group: string) => `${group} grubunu aç`,
    notReady: 'Geliştirme aşamasında',
  },
} as const;
