import type { Profile } from '../../contracts/generated/sheet/Profile';
import type { ToolInfo } from '../../contracts/generated/sheet/ToolInfo';
import type { ItemKindView } from './view';

/**
 * A work mode's sheet profile as the interface reads it (docs/sheet/design.md
 * §11a): the engine's `Profile` for the project's mode and capabilities
 * (`profileFor`, crates/shared/sheet/data/profiles.json): the Pafta tab's
 * tool groups, each tool's name in the mode, whether it is shown, shown off
 * (with the reason) or hidden, its ready looks (presets), the item kinds'
 * names in the mode, the default template of a new sheet and the gallery's
 * order. The interface decides nothing by mode in code: what it adds here
 * is only how things look (an icon per kind and per ready look) and the
 * names of the kinds no tool of the mode names.
 */

export type SheetProfile = Profile;

/** What the interface shows before the engine has answered: no tools to add, nothing named. */
export const NO_PROFILE: SheetProfile = { id: '', label: '', groups: [], names: {}, defaultTemplate: '', gallery: [] };

/** A kind's name when no tool of the mode names it (a group, an item of a hidden tool). */
export const COMMON_NAMES: Readonly<Record<ItemKindView, string>> = {
  map: 'Harita',
  text: 'Metin',
  scaleBar: 'Ölçek çubuğu',
  northArrow: 'Kuzey oku',
  legend: 'Lejant',
  picture: 'Resim',
  shape: 'Şekil',
  line: 'Çizgi',
  table: 'Tablo',
  coordinateList: 'Koordinat listesi',
  titleBlock: 'Antet',
  border: 'Pafta çerçevesi',
  group: 'Grup',
};

/** The icon of an item kind, in the tree, the inspector and the gallery. */
export const KIND_ICON: Readonly<Record<ItemKindView, string>> = {
  map: 'sheetMap',
  text: 'text',
  scaleBar: 'sheetScaleBar',
  northArrow: 'sheetNorth',
  legend: 'legend',
  picture: 'sheetPicture',
  shape: 'sheetShape',
  line: 'sheetLine',
  table: 'table',
  coordinateList: 'sheetCoordinates',
  titleBlock: 'sheetTitleBlock',
  border: 'sheetBorder',
  group: 'sheetGroup',
};

/** Icons of the tools that add no item, and of the ones whose look differs from their kind's. */
const TOOL_ICON: Readonly<Record<string, string>> = {
  select: 'select',
  pan: 'pan',
  grid: 'sheetGrid',
  atlas: 'sheetAtlas',
  group: 'sheetGroup',
  overviewMap: 'sheetOverview',
};

/** Icons of ready looks that are a picture of their own (Şekil ▾, Çizgi ▾, Ölçek ▾, Çerçeve ▾, Tablo ▾, Kuzey ▾, Harita ▾, Lejant ▾). */
const PRESET_ICON: Readonly<Record<string, string>> = {
  rect: 'sheetRect',
  rounded: 'sheetRounded',
  ellipse: 'sheetEllipse',
  triangle: 'sheetTriangle',
  polygon: 'sheetPolygon',
  plain: 'sheetLinePlain',
  arrow: 'sheetArrow',
  numeric: 'sheetScaleNumeric',
  neat: 'sheetBorderNeat',
  revisions: 'sheetTableRevisions',
  drawings: 'sheetTableDrawings',
  compass: 'sheetCompass',
  viewport: 'sheetViewport',
  layers: 'sheetLegendLayers',
  thematic: 'sheetLegendThematic',
};

const isKind = (k: string | undefined): k is ItemKindView => !!k && k in KIND_ICON;

export function toolIcon(tool: ToolInfo): string {
  return TOOL_ICON[tool.id] ?? (isKind(tool.item) ? KIND_ICON[tool.item] : 'sheetShape');
}

export function presetIcon(tool: ToolInfo, preset: string): string {
  return PRESET_ICON[preset] ?? toolIcon(tool);
}

/** Every tool of the profile, flat, in its groups' order. */
export const toolsOf = (profile: SheetProfile): ToolInfo[] => profile.groups.flatMap((g) => g.tools);

/** The tools that add an item (Ekle). */
export const addTools = (profile: SheetProfile): ToolInfo[] => toolsOf(profile).filter((t) => !!t.item);

/** An item kind's name in the mode: the mode's own word, else its first tool's, else the common one. */
export function kindName(profile: SheetProfile, kind: ItemKindView): string {
  return profile.names[kind] ?? toolsOf(profile).find((t) => t.item === kind)?.label ?? COMMON_NAMES[kind];
}
