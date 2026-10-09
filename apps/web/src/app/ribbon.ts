import type { CategoryNode } from '../processing/registry';
import type { ToolDescriptor, ToolGroup } from '../tools/Tool';
import { toolSections } from '../tools/sections';
import { menuBlocks, menuById, type MenuBlock, type SubmenuSpec } from './menus';
import { POINT_CLOUD_TOOLS, POINT_CLOUDS } from './pointCloudCommands';
import { modelCommandId, processingCommandId } from './processing';
import { SHOW_ALL, type WorkspaceFilter } from './workspaces';
import type { Workspace } from '../model/projectSettings';

/**
 * The ribbon (Şerit) as data. Tabs are composed from the main menu
 * (app/menus.ts) and the tool catalog; they never list tools or menu
 * commands one by one, so anything added there shows up here by itself.
 * Only the Giriş tab hand-picks the everyday tools (each also lives in its
 * own tab) and the quick access bar starts with save, undo and redo.
 *
 * Each project type has its own ribbon (docs/adr/0165 §6): a CAD project
 * AutoCAD's 2D drafting tabs (CAD_RIBBON_TABS), a CBS project the map
 * work's (GIS_RIBBON_TABS). RIBBON_TABS is the ribbon with no type, every
 * menu as a tab: what the composition rules are tested on (ribbon.test.ts)
 * and where the contextual Seçim tab is defined; no project shows it.
 */
export type RibbonSource =
  /** The titled blocks of a main menu, one panel each (optionally only the named ones). */
  | { readonly menu: string; readonly sections?: readonly string[] }
  /** A tool group from the catalog, one panel per tool section. */
  | { readonly tools: ToolGroup }
  /**
   * A hand-picked panel; `more` names the tab that holds the whole family,
   * `compact` draws every button small (AutoCAD's Modify panel), and
   * `workspaces` shows it only in those work modes. `rest` puts the other
   * tools of a group (`draw`) or a group's section (`map/measure`) under the
   * panel's ▾, as AutoCAD's panel expander; `under` puts commands there.
   */
  | {
      readonly pick: string;
      readonly icon: string;
      readonly commands: readonly string[];
      readonly more?: string;
      readonly compact?: boolean;
      readonly workspaces?: readonly Workspace[];
      readonly rest?: string;
      readonly under?: readonly string[];
    }
  /** Panels with live fields rather than commands. */
  /** A panel with live fields (BuiltinPanel); `under` puts commands under its ▾ (Katmanlar's layer actions, docs/adr/0177 §7). */
  | { readonly builtin: BuiltinPanel; readonly under?: readonly string[] };

export type BuiltinPanel = 'layers' | 'properties' | 'selection' | 'templates';

export interface RibbonTabSpec {
  readonly id: string;
  readonly label: string;
  /** The tab's name in a work mode where what is left of it is better called otherwise. */
  readonly labels?: Partial<Record<Workspace, string>>;
  readonly sources: readonly RibbonSource[];
  /** Commands that lead their panel (moved to its front). */
  readonly lead?: readonly string[];
  /** Panels that keep their labels longest when the window narrows (by title). */
  readonly keep?: readonly string[];
  /** A contextual tab: shown only while its subject exists (a selection under the select tool). */
  readonly contextual?: 'selection';
  /** Dialog launchers (the corner button of a panel title), by panel label. */
  readonly launchers?: Readonly<Record<string, RibbonLauncher>>;
}

export type RibbonLauncher = { readonly command: string; readonly args?: unknown; readonly title: string } | { readonly tab: string; readonly title: string };

/**
 * Commands other than tools that are the main thing of their panel, drawn
 * large wherever they appear (tools say so in the catalog, `primary`).
 */
export const PRIMARY_COMMANDS: ReadonlySet<string> = new Set([
  'file.new',
  'file.open',
  'file.save',
  'cloud.open',
  'cloud.upload',
  'edit.paste',
  'view.zoomExtents',
  'crs.set',
  'processing.toolbox',
  'style.manager',
  'style.svgEditor',
  'tools.options',
]);

/** Large buttons a panel may have; more would crowd out the small ones. */
export const MAX_LARGE = 4;

/** The layer actions by an object, Yalıtımı kaldır, Katmanı eşle, Katmana kopyala and Katmanları birleştir (docs/adr/0177 §1–§3), under Katmanlar's ▾ in every ribbon. */
const LAYER_ACTIONS = ['tool.layerOff', 'tool.layerIsolate', 'layer.unisolate', 'tool.layerLock', 'tool.layerMakeActive', 'tool.layerMatch', 'tool.copyToLayer', 'layer.merge', 'layer.states', 'layer.stateSave', 'layer.purge', 'layer.list'];

export const RIBBON_TABS: readonly RibbonTabSpec[] = [
  { id: 'file', label: 'Dosya', sources: [{ menu: 'file' }] },
  {
    id: 'home',
    label: 'Giriş',
    sources: [
      { menu: 'edit', sections: ['Pano'] },
      { tools: 'select' },
      { menu: 'edit', sections: ['Seçim'] },
      { pick: 'Çizim', icon: 'line', commands: ['tool.line', 'tool.polyline', 'tool.circle', 'tool.arc', 'tool.polygon', 'tool.adjoin', 'tool.rectangle'], more: 'draw' },
      {
        pick: 'Değiştir',
        icon: 'move',
        commands: ['tool.move', 'tool.copy', 'tool.rotate', 'tool.mirror', 'tool.scale', 'tool.offset', 'tool.trim', 'tool.extend', 'tool.fillet'],
        more: 'modify',
        compact: true,
      },
      { pick: 'Açıklama', icon: 'text', commands: ['tool.text', 'tool.dimension', 'tool.hatch', 'text.findReplace'], more: 'draw' },
      { pick: 'Harita', icon: 'parcel', commands: ['tool.parcel', 'tool.boundary', 'tool.areaUnion', 'tool.measure', 'tool.area'], more: 'map', workspaces: ['gis'] },
      { builtin: 'layers', under: LAYER_ACTIONS },
      { builtin: 'templates' },
      { builtin: 'properties' },
    ],
    lead: ['edit.paste'],
    keep: ['Çizim', 'Değiştir'],
    launchers: {
      Katmanlar: { command: 'style.layerStyle', title: 'Katman stili…' },
      Şablonlar: { command: 'template.panel', title: 'Şablonlar paneli' },
      Özellikler: { command: 'file.settings', title: 'Proje ayarları: birimler, hassasiyet ve çizim ölçeği' },
    },
  },
  { id: 'draw', label: 'Çizim', sources: [{ menu: 'draw' }] },
  { id: 'modify', label: 'Değiştir', sources: [{ menu: 'modify' }] },
  {
    id: 'map',
    label: 'Harita',
    // CAD shows no map or coordinate menus: what is left is measuring.
    labels: { cad: 'Ölçme' },
    sources: [
      { menu: 'map' },
      { menu: 'calc' },
      { menu: 'crs' },
      { menu: 'analysis' },
      // Öznitelik tablosu, a layer's fields and the data sources (docs/adr/0199 §3, §4, §7).
      { pick: 'Tablo', icon: 'featureTable', commands: ['data.featureTable', 'layer.fields', 'data.sources'] },
    ],
    launchers: { 'Koordinat sistemi': { command: 'crs.set', title: 'Proje ayarları: koordinat sistemi' } },
  },
  {
    id: 'view',
    label: 'Görünüm',
    sources: [{ menu: 'view' }],
    launchers: { Görünüş: { command: 'tools.options', args: 'appearance', title: 'Uygulama ayarları: görünüm' } },
  },
  { id: 'processing', label: 'İşlemler', sources: [{ menu: 'processing' }] },
  {
    id: 'tools',
    label: 'Araçlar',
    sources: [{ menu: 'tools' }, { menu: 'help' }],
    launchers: { 'Çizim yardımcıları': { command: 'tools.options', args: 'snap', title: 'Uygulama ayarları: kenetleme' } },
  },
  {
    id: 'selection',
    label: 'Seçim',
    contextual: 'selection',
    sources: [
      { builtin: 'selection' },
      { menu: 'modify', sections: ['Dönüştür', 'Dizi', 'Nesne', 'Birleştir ve böl', 'Oluştur ve çevir'] },
      { menu: 'edit', sections: ['Pano'] },
      { pick: 'Sembol', icon: 'symbolAssign', commands: ['style.assign', 'style.clearSymbol'] },
    ],
  },
];

/** Giriş's clipboard and selection panels, the same in every ribbon. */
const CLIPBOARD_AND_SELECTION: readonly RibbonSource[] = [{ menu: 'edit', sections: ['Pano'] }, { tools: 'select' }, { menu: 'edit', sections: ['Seçim'] }];
/** The everyday drawing and modifying tools of Giriş. */
const EVERYDAY_DRAW = ['tool.line', 'tool.polyline', 'tool.circle', 'tool.arc', 'tool.polygon', 'tool.adjoin', 'tool.rectangle'];
/** A CAD project's Giriş has no drawing tab beside it: Elips, Eğri and Nokta show too (AutoCAD's Draw panel). */
const CAD_DRAW = [...EVERYDAY_DRAW, 'tool.ellipse', 'tool.spline', 'tool.point'];
const EVERYDAY_MODIFY = ['tool.move', 'tool.copy', 'tool.rotate', 'tool.mirror', 'tool.scale', 'tool.offset', 'tool.trim', 'tool.extend', 'tool.fillet'];
const HOME_LAUNCHERS: Readonly<Record<string, RibbonLauncher>> = {
  Katmanlar: { command: 'style.layerStyle', title: 'Katman stili…' },
  Şablonlar: { command: 'template.panel', title: 'Şablonlar paneli' },
  Özellikler: { command: 'file.settings', title: 'Proje ayarları: birimler, hassasiyet ve çizim ölçeği' },
};
const VIEW_LAUNCHERS: Readonly<Record<string, RibbonLauncher>> = { Görünüş: { command: 'tools.options', args: 'appearance', title: 'Uygulama ayarları: görünüm' } };
const AIDS_LAUNCHER: Readonly<Record<string, RibbonLauncher>> = { 'Çizim yardımcıları': { command: 'tools.options', args: 'snap', title: 'Uygulama ayarları: kenetleme' } };
const IMPORTS = ['file.import.dxf', 'file.import.ncz', 'file.import.shp', 'file.import.geojson', 'file.import.ncn', 'file.import.gnss'];
const EXPORTS = ['file.export.dxf', 'file.export.pdf', 'file.export.geojson', 'file.export.ncn'];
const SHEET_LAYOUTS = ['sheet.new', 'sheet.fromTemplate'];
const SELECTION_TAB: RibbonTabSpec = RIBBON_TABS.find((t) => t.contextual === 'selection')!;

/** Raster katmanları (docs/adr/0204 §8): CAD's Ekle › Raster and CBS's Veri › Raster. */
const RASTERS = ['raster.add', 'raster.style', 'raster.georef'];
/** Nokta bulutu (docs/adr/0207 §9), the desktop's for now: CAD's Ekle and CBS's Veri, İşlemler's tools under ▾. */
const POINT_CLOUD_PANEL = { pick: 'Nokta bulutu', icon: 'pointCloudAdd', commands: POINT_CLOUDS, under: POINT_CLOUD_TOOLS } as const;

/**
 * A CAD project's ribbon (docs/adr/0165 §6), AutoCAD's 2D drafting tabs: Giriş with drawing, modifying, annotation,
 * layers, blocks, properties and measuring (the other drawing tools and the survey computations under their panels'
 * ▾); Ekle with blocks and imports; Açıklama with text, dimensions, leaders, hatches and markup; Yönet with cleaning,
 * styles and the commands; Çıktı with sheets, printing and exports.
 */
export const CAD_RIBBON_TABS: readonly RibbonTabSpec[] = [
  { id: 'file', label: 'Dosya', sources: [{ menu: 'file' }] },
  {
    id: 'home',
    label: 'Giriş',
    sources: [
      ...CLIPBOARD_AND_SELECTION,
      { pick: 'Çizim', icon: 'line', commands: CAD_DRAW, rest: 'draw' },
      { pick: 'Değiştir', icon: 'move', commands: EVERYDAY_MODIFY, more: 'modify', compact: true },
      { pick: 'Açıklama', icon: 'text', commands: ['tool.text', 'tool.dimension', 'tool.leader', 'tool.hatch'], more: 'annotate' },
      { builtin: 'layers', under: LAYER_ACTIONS },
      { builtin: 'templates' },
      { pick: 'Blok', icon: 'blockInsert', commands: ['tool.blockInsert', 'tool.blockDefine', 'block.panel'], more: 'insert' },
      { builtin: 'properties' },
      { pick: 'Ölçme', icon: 'measure', commands: ['tool.measure', 'tool.area', 'tool.measureAngle', 'tool.stationOffset', 'crs.query'], under: ['calc.fieldbook', 'file.import.gnss', 'field.send', 'calc.traverse', 'calc.polar', 'calc.stakeout', 'calc.forward', 'calc.resection', 'calc.network', 'calc.levelNetwork'] },
    ],
    lead: ['edit.paste'],
    keep: ['Çizim', 'Değiştir'],
    launchers: HOME_LAUNCHERS,
  },
  {
    id: 'insert',
    label: 'Ekle',
    sources: [
      { menu: 'draw', sections: ['Blok'] },
      // Resim (docs/adr/0192 §5): AutoCAD's Insert › Reference.
      { menu: 'draw', sections: ['Resim'] },
      // Raster (docs/adr/0204 §8): Netcad's Raster Yükle, AutoCAD Map's Insert › Image.
      { pick: 'Raster', icon: 'rasterAdd', commands: RASTERS },
      POINT_CLOUD_PANEL,
      // Harita altlığı (docs/adr/0208 §14): a ready basemap or a map service under the drawing, AutoCAD Map's Connect.
      { menu: 'map', sections: ['Altlık'] },
      { pick: 'İçe aktar', icon: 'import', commands: IMPORTS },
      // Çizimler arası alışveriş (docs/adr/0193): AutoCAD's DesignCenter and WBLOCK.
      { pick: 'Alışveriş', icon: 'takeFrom', commands: ['file.takeFrom', 'file.saveSelection'] },
      { pick: 'Metin', icon: 'textFile', commands: ['tool.placeTextFile'] },
    ],
    // Blok ekle leads its panel, as AutoCAD's Insert.
    lead: ['tool.blockInsert'],
  },
  {
    id: 'annotate',
    label: 'Açıklama',
    // AutoCAD's Annotate tab, a panel a kind; a tool the draw menu's Açıklama block gains later shows in a panel of its own.
    sources: [
      // Eğri boyunca yazı (docs/adr/0196 §4) beside the texts.
      { pick: 'Yazı', icon: 'text', commands: ['tool.text', 'tool.mtext', 'tool.textAlong', 'tool.textCurve', 'tool.placeTextFile', 'tool.labelsToText', 'text.findReplace', 'style.textStyles'] },
      { pick: 'Ölçü', icon: 'dimension', commands: ['tool.dimension', 'style.dimensionStyles'] },
      // Koordinat yaz (docs/adr/0185): a place's coordinates on the drawing, and every corner's with their schedule.
      { pick: 'Koordinat', icon: 'coordinateLabel', commands: ['tool.coordinateLabel', 'tool.coordinateVertices'] },
      // Km yaz (docs/adr/0189): a route's stations, their ticks, km texts, cross-sections and points.
      { pick: 'Km', icon: 'stationLabels', commands: ['tool.stationLabels'] },
      { pick: 'Kılavuz', icon: 'leader', commands: ['tool.leader'] },
      // Tablo (docs/adr/0184): AutoCAD's Annotate › Tables.
      { pick: 'Tablo', icon: 'table', commands: ['table.insert', 'table.edit', 'table.update'] },
      // Tarama ekleri (docs/adr/0186): Çoklu tara beside it.
      { pick: 'Tarama', icon: 'hatch', commands: ['tool.hatch', 'tool.hatchSelected'] },
      { pick: 'İşaretleme', icon: 'revcloud', commands: ['tool.revcloud'] },
      { menu: 'draw', sections: ['Açıklama'] },
      { menu: 'edit', sections: ['Bul'] },
    ],
  },
  { id: 'modify', label: 'Değiştir', sources: [{ menu: 'modify' }] },
  { id: 'view', label: 'Görünüm', sources: [{ menu: 'view' }], launchers: VIEW_LAUNCHERS },
  {
    id: 'manage',
    label: 'Yönet',
    sources: [
      { pick: 'Temizlik', icon: 'cleanup', commands: ['tool.cleanup', 'tool.topology', 'block.purge', 'layer.purge'] },
      // Topoloji kuralları (docs/adr/0202 §8): the rules and their check beside the cleaning.
      { pick: 'Topoloji', icon: 'topologyCheck', commands: ['topology.check', 'topology.rules'] },
      // Öznitelik tablosu and a layer's fields (docs/adr/0199 §3, §4).
      { pick: 'Tablo', icon: 'featureTable', commands: ['data.featureTable', 'layer.fields', 'data.sources'] },
      { menu: 'analysis', sections: ['Karşılaştırma'] },
      { menu: 'calc', sections: ['Kayıtlı ölçüler'] },
      { menu: 'tools' },
      { menu: 'help' },
    ],
    launchers: AIDS_LAUNCHER,
  },
  {
    id: 'output',
    label: 'Çıktı',
    sources: [
      { pick: 'Pafta', icon: 'sheet', commands: SHEET_LAYOUTS },
      { menu: 'file', sections: ['Çıktı'] },
      { pick: 'Dışa aktar', icon: 'export', commands: EXPORTS },
    ],
  },
  SELECTION_TAB,
];

/**
 * A CBS project's ribbon (docs/adr/0165 §6), after ArcGIS Pro, QGIS and Netcad: Harita with navigation, the
 * coordinate system, measuring, parcels and styles; Veri with layers, imports and exports, coordinates, attributes and
 * blocks;
 * Düzenle with creating and modifying objects and the editing aids; Analiz with the processing tools, models, terrain
 * analysis and the command line; Ölçme with the survey computations and points; Çıktı with sheets, the legend and exports.
 */
export const GIS_RIBBON_TABS: readonly RibbonTabSpec[] = [
  { id: 'file', label: 'Dosya', sources: [{ menu: 'file' }] },
  {
    id: 'home',
    label: 'Giriş',
    sources: [
      ...CLIPBOARD_AND_SELECTION,
      { pick: 'Çizim', icon: 'line', commands: EVERYDAY_DRAW, more: 'edit' },
      { pick: 'Değiştir', icon: 'move', commands: EVERYDAY_MODIFY, more: 'edit', compact: true },
      { pick: 'Açıklama', icon: 'text', commands: ['tool.text', 'text.findReplace'], more: 'edit' },
      { pick: 'Harita', icon: 'parcel', commands: ['tool.parcel', 'tool.boundary', 'tool.areaUnion', 'tool.measure', 'tool.area'], more: 'map' },
      { builtin: 'layers', under: LAYER_ACTIONS },
      { builtin: 'templates' },
      { builtin: 'properties' },
    ],
    lead: ['edit.paste'],
    keep: ['Çizim', 'Değiştir'],
    launchers: HOME_LAUNCHERS,
  },
  {
    id: 'map',
    label: 'Harita',
    sources: [
      { menu: 'view', sections: ['Yakınlaştır'] },
      { menu: 'crs', sections: ['Koordinat sistemi'] },
      // Altlık (docs/adr/0208 §14): ArcGIS Pro's Basemap and Add Data, QGIS's XYZ, WMS/WMTS and vector tiles.
      { menu: 'map', sections: ['Altlık'] },
      { menu: 'map', sections: ['Parsel', 'Ölçme'] },
      // Plan yolu çizimi (docs/adr/0198 §5): the roads beside the parcels.
      { pick: 'Yol', icon: 'planRoad', commands: ['tool.planRoad', 'tool.roadJunctions', 'tool.medianClose'] },
      { menu: 'tools', sections: ['Stil'] },
      // The layers' labels as texts (docs/adr/0175 §3), beside the styles that draw them; a name along a creek or a
      // road (docs/adr/0196 §4).
      { pick: 'Etiket', icon: 'labelsToText', commands: ['tool.labelsToText', 'tool.textAlong', 'tool.textCurve'] },
    ],
    launchers: { 'Koordinat sistemi': { command: 'crs.set', title: 'Proje ayarları: koordinat sistemi' } },
  },
  {
    id: 'data',
    label: 'Veri',
    sources: [
      { pick: 'Katman', icon: 'layerAdd', commands: ['layer.new', 'layer.newGroup', 'layer.showAll'] },
      // Öznitelik tablosu, a layer's fields and the data sources (docs/adr/0199 §3, §4, §7).
      { pick: 'Tablo', icon: 'featureTable', commands: ['data.featureTable', 'layer.fields', 'data.sources'] },
      // Veride ara (docs/adr/0178): a value in the layers' data, and a place by its coordinates.
      { pick: 'Ara', icon: 'dataSearch', commands: ['data.search', 'data.unmark'] },
      { menu: 'file', sections: ['Dosya alışverişi'] },
      { menu: 'crs', sections: ['Koordinatlar'] },
      { pick: 'Öznitelik', icon: 'fieldCalc', commands: [processingCommandId('attributes.calculate'), processingCommandId('selection.byExpression')] },
      // The drawing's block definitions are its library, as its layers and attributes are; its pictures beside them (docs/adr/0192 §5).
      { menu: 'draw', sections: ['Blok', 'Resim'] },
      // Raster katmanları (docs/adr/0204 §8): QGIS's Add Raster Layer and Georeferencer.
      { pick: 'Raster', icon: 'rasterAdd', commands: RASTERS },
      // Nokta bulutu (docs/adr/0207 §9): QGIS's Add Point Cloud Layer and its PDAL tools.
      POINT_CLOUD_PANEL,
    ],
    lead: ['tool.blockInsert'],
  },
  {
    id: 'edit',
    label: 'Düzenle',
    sources: [
      { menu: 'draw', sections: ['Çizgi', 'Eğri', 'Şekil', 'Yardımcı', 'Nokta', 'Açıklama'] },
      { menu: 'modify' },
      { menu: 'tools', sections: ['Çizim yardımcıları'] },
    ],
    launchers: AIDS_LAUNCHER,
  },
  {
    id: 'analysis',
    label: 'Analiz',
    sources: [
      { menu: 'processing' },
      { menu: 'map', sections: ['Arazi'] },
      { menu: 'analysis', sections: ['Arazi analizi'] },
      // Topoloji kuralları (docs/adr/0202 §8): the data's checks together, beside Karşılaştırma (Düzenle is full).
      { pick: 'Topoloji', icon: 'topologyCheck', commands: ['topology.check', 'topology.rules'] },
      { menu: 'analysis', sections: ['Karşılaştırma'] },
      { menu: 'tools', sections: ['Komut'] },
    ],
  },
  {
    id: 'survey',
    label: 'Ölçme',
    sources: [
      { menu: 'calc' },
      { pick: 'Noktalar', icon: 'point', commands: ['tool.point', 'tool.vertexPoints', 'point.editor'] },
      { pick: 'Oturtma', icon: 'vectorFit', commands: ['transform.fit', 'transform.edgematch'] },
    ],
  },
  { id: 'view', label: 'Görünüm', sources: [{ menu: 'view' }, { menu: 'tools', sections: ['Uygulama'] }, { menu: 'help' }], launchers: VIEW_LAUNCHERS },
  {
    id: 'output',
    label: 'Çıktı',
    sources: [
      { pick: 'Pafta', icon: 'sheet', commands: SHEET_LAYOUTS },
      { menu: 'map', sections: ['Pafta'] },
      { pick: 'Lejant', icon: 'legend', commands: ['style.legend'] },
      { menu: 'file', sections: ['Çıktı'] },
      { pick: 'Dışa aktar', icon: 'export', commands: EXPORTS },
    ],
  },
  SELECTION_TAB,
];

/** A project type's ribbon; none: the ribbon with no type (RIBBON_TABS). */
export function ribbonSpecs(type: Workspace | null | undefined): readonly RibbonTabSpec[] {
  return type === 'cad' ? CAD_RIBBON_TABS : type === 'gis' ? GIS_RIBBON_TABS : RIBBON_TABS;
}

/** Always on the quick access bar; the user may add more (and remove what they added). */
export const QUICK_ACCESS: readonly string[] = ['file.save', 'edit.undo', 'edit.redo'];

/**
 * The quick access bar from the layout's kept list (`ribbonQuickAccess`,
 * app/layoutPlan.ts): the fixed commands, then the ones the user added that
 * this app has, once each, in the order added.
 */
export function quickAccessOf(kept: readonly string[], exists: (id: string) => boolean): string[] {
  return [...QUICK_ACCESS, ...new Set(kept.filter((id) => exists(id) && !QUICK_ACCESS.includes(id)))];
}

/** The ribbon's tab at start: the kept one (`ribbonTab`) while the work mode shows it and it is not contextual; else Giriş. */
export function startTab(kept: string, tabs: readonly { readonly id: string; readonly contextual?: string }[]): string {
  return tabs.some((t) => t.id === kept && !t.contextual) ? kept : 'home';
}

/** One choice of a split button: a tool, or a tool started with one of its methods. */
export interface SplitEntry {
  readonly command: string;
  /** The prompt option sent right after the tool starts (ToolMethod.option). */
  readonly option?: string;
  /** The button's text: the tool's name (a method does not rename it). */
  readonly title: string;
  /** The list's text: the method, or the tool's name. */
  readonly label: string;
  readonly description?: string;
  /** Typed names that start the tool with this method (ToolMethod.aliases). */
  readonly aliases?: readonly string[];
  /** The method's own icon (ToolMethod.icon); none: its command's. */
  readonly icon?: string;
}

/** A split choice as the layout keeps it (`ribbonSplits`, by the button's key): its command and its method's option. */
export const splitChoiceKey = (e: Pick<SplitEntry, 'command' | 'option'>): string => `${e.command}|${e.option ?? ''}`;

/** A split button's entry on top: the one last chosen, or its first when none was or the kept one is gone. */
export function splitCurrent<E extends Pick<SplitEntry, 'command' | 'option'>>(entries: readonly E[], kept: string | undefined): E {
  return entries.find((e) => splitChoiceKey(e) === kept) ?? entries[0];
}

export type RibbonItem =
  | { readonly kind: 'command'; readonly id: string; readonly size: RibbonSize }
  /** A family of tools, or a tool's methods: the last chosen on top, the others under its arrow. */
  | { readonly kind: 'split'; readonly key: string; readonly entries: readonly SplitEntry[]; readonly size: RibbonSize }
  | { readonly kind: 'menu'; readonly menu: SubmenuSpec; readonly size: RibbonSize }
  | { readonly kind: 'builtin'; readonly name: BuiltinPanel };

/** Large, small, or a panel's lead: large while the panel shows labels, small only among icons (DESIGN.md §7.3.1). */
export type RibbonSize = 'large' | 'lead' | 'small';

export interface RibbonPanel {
  readonly label: string;
  readonly icon: string;
  readonly items: readonly RibbonItem[];
  readonly launcher?: RibbonLauncher;
  /** Seldom used commands of the panel, listed under the ▾ beside its title. */
  readonly overflow?: readonly string[];
  /** Shrinks after the other panels of its tab. */
  readonly keep?: boolean;
}

export interface RibbonTab {
  readonly id: string;
  readonly label: string;
  /** Shown only while its subject exists: a selection, or a sheet in front (the sheet layouts' Pafta tab, ui/ribbon/Ribbon.ts `extend`). */
  readonly contextual?: 'selection' | 'sheet';
  readonly panels: readonly RibbonPanel[];
}

/** What the tabs are derived from: the registered tools, the processing tree and the model library. */
export interface RibbonInputs {
  readonly tools: readonly ToolDescriptor[];
  readonly processing: readonly CategoryNode[];
  readonly models: readonly { readonly id: string }[];
  /** Icon of a command (panel icons follow their first item). */
  readonly iconOf: (id: string) => string | undefined;
  /** The project's work mode (everything when absent). */
  readonly filter?: WorkspaceFilter;
}

type Entry = { kind: 'command'; id: string } | { kind: 'menu'; menu: SubmenuSpec } | { kind: 'builtin'; name: BuiltinPanel };
type Draft = { label: string; icon?: string; entries: Entry[]; more?: string; compact?: boolean; under: string[]; picked: Set<string> };

const BUILTIN_LABEL: Record<BuiltinPanel, { label: string; icon: string }> = {
  layers: { label: 'Katmanlar', icon: 'layers' },
  properties: { label: 'Özellikler', icon: 'styles' },
  selection: { label: 'Seçim', icon: 'select' },
  templates: { label: 'Şablonlar', icon: 'templates' },
};

/**
 * Tabs with their panels, sized for their widest layout. Within a tab
 * panels with the same title merge and a command appears once (its first
 * place wins); the ribbon then shrinks panels to fit the window.
 *
 * Sizes follow meaning, not position (DESIGN.md §7.3.1): a panel's main
 * tools and commands (`primary` in the catalog, PRIMARY_COMMANDS) are
 * large, at most MAX_LARGE; the rest are small, three to a column; a panel
 * with no main item and one or two items draws them large; a `compact`
 * pick draws everything small. A tool family (`family`) is one split
 * button, and so is a tool with `methods`; `rare` tools go under the
 * panel's ▾. The work mode (`inputs.filter`) leaves out what it hides.
 */
export function ribbonTabs(inputs: RibbonInputs, specs: readonly RibbonTabSpec[] = ribbonSpecs(inputs.filter?.id)): RibbonTab[] {
  const filter = inputs.filter ?? SHOW_ALL;
  const tools = inputs.tools.filter((t) => filter.tool(t));
  const toolOf = new Map(tools.map((t) => [`tool.${t.id}`, t]));
  const family = (f: string) => tools.filter((t) => t.family === f);
  const tabs = specs.map((spec) => {
    const drafts: Draft[] = [];
    const seen = new Set<string>();
    const panel = (label: string, icon?: string, more?: string, compact?: boolean): Draft => {
      const found = drafts.find((d) => d.label === label);
      if (found) return found;
      const d: Draft = { label, icon, entries: [], more, compact, under: [], picked: new Set() };
      drafts.push(d);
      return d;
    };
    const command = (d: Draft, id: string) => {
      if (seen.has(id) || !filter.command(id)) return;
      const t = toolOf.get(id);
      // A family travels together: its other members join the first one's panel.
      const ids = t?.family ? family(t.family).map((m) => `tool.${m.id}`) : [id];
      for (const x of ids) {
        if (seen.has(x)) continue;
        seen.add(x);
        d.entries.push({ kind: 'command', id: x });
      }
    };
    for (const src of spec.sources) {
      if ('builtin' in src) {
        const b = BUILTIN_LABEL[src.builtin];
        const d = panel(b.label, b.icon);
        d.entries.push({ kind: 'builtin', name: src.builtin });
        for (const id of src.under ?? []) {
          if (seen.has(id) || !filter.command(id)) continue;
          seen.add(id);
          d.under.push(id);
        }
      } else if ('pick' in src) {
        // A type's own panel: left out where no type filters (the inventory's places).
        if (src.workspaces && (filter.id === null || !src.workspaces.includes(filter.id))) continue;
        const d = panel(src.pick, src.icon, src.more, src.compact);
        for (const id of src.commands) {
          d.picked.add(id);
          command(d, id);
        }
        // Under the ▾: the group's other tools (AutoCAD's panel expander) and the commands named.
        const rest = src.rest ? tools.filter((t) => t.group === src.rest || `${t.group}/${t.section ?? ''}` === src.rest).map((t) => `tool.${t.id}`) : [];
        for (const id of [...rest, ...(src.under ?? [])]) {
          if (seen.has(id) || !filter.command(id)) continue;
          seen.add(id);
          d.under.push(id);
        }
      } else if ('tools' in src) {
        for (const s of toolSections(tools, src.tools)) {
          const d = panel(s.label);
          for (const t of s.tools) command(d, `tool.${t.id}`);
        }
      } else {
        const menu = menuById(src.menu);
        if (!menu || !filter.menu(menu.id)) continue;
        const keep = (label: string) => !src.sections || src.sections.includes(label);
        for (const block of expandBlocks(menuBlocks(menu.items, tools, filter), menu.label, inputs, tools, filter)) {
          if (!keep(block.label)) continue;
          const d = panel(block.label);
          for (const e of block.entries) {
            if (e.kind === 'command') command(d, e.id);
            else d.entries.push(e);
          }
        }
      }
    }
    const lead = new Set(spec.lead ?? []);
    const leads = (e: Entry) => e.kind === 'command' && lead.has(e.id);
    const panels = drafts
      .filter((d) => d.entries.length || d.under.length)
      .map((d): RibbonPanel => {
        const ordered = [...d.entries.filter(leads), ...d.entries.filter((e) => !leads(e))];
        // A pick shows what it names, seldom used or not (Yönet's Temizlik, Açıklama's İşaretleme).
        const rare = (e: Entry) => e.kind === 'command' && !d.picked.has(e.id) && !!toolOf.get(e.id)?.rare;
        // Rare tools go under the ▾, unless nothing would be left on the panel; and what the pick put there.
        const overflow = [...(ordered.every(rare) ? [] : ordered.filter(rare).map((e) => (e.kind === 'command' ? e.id : ''))), ...d.under];
        const shown = groupSplits(ordered.filter((e) => !overflow.includes(e.kind === 'command' ? e.id : '')), toolOf);
        const primary = (x: Pre) =>
          x.kind === 'command' ? !!toolOf.get(x.id)?.primary || PRIMARY_COMMANDS.has(x.id) : x.kind === 'split' ? x.members.some((m) => m.primary) : x.kind === 'menu' ? !!x.menu.primary : false;
        const anyPrimary = shown.some(primary);
        const lead = (x: Pre) => (x.kind === 'command' ? !!toolOf.get(x.id)?.lead : x.kind === 'split' ? x.members.some((m) => m.lead) : false);
        const items = shown.map((x): RibbonItem => {
          if (x.kind === 'builtin') return x;
          const large = !d.compact && (primary(x) || (!anyPrimary && shown.length <= 2));
          const size: RibbonSize = large ? (lead(x) ? 'lead' : 'large') : 'small';
          if (x.kind === 'command') return { kind: 'command', id: x.id, size };
          if (x.kind === 'menu') return { kind: 'menu', menu: x.menu, size };
          return { kind: 'split', key: x.key, entries: x.entries, size };
        });
        const first: Entry = ordered[0] ?? { kind: 'command', id: d.under[0] };
        const icon = d.icon ?? (first.kind === 'command' ? inputs.iconOf(first.id) : first.kind === 'menu' ? first.menu.icon : undefined) ?? 'more';
        const launcher = spec.launchers?.[d.label] ?? (d.more ? { tab: d.more, title: `Tüm araçlar: ${specs.find((s) => s.id === d.more)?.label ?? d.more} sekmesi` } : undefined);
        return { label: d.label, icon, items, launcher, overflow: overflow.length ? overflow : undefined, keep: spec.keep?.includes(d.label) || undefined };
      });
    return { id: spec.id, label: (filter.id && spec.labels?.[filter.id]) || spec.label, contextual: spec.contextual, panels };
  });
  // A tab the work mode leaves empty (İşlemler in CAD) is not shown.
  return tabs.filter((t) => t.panels.length);
}

/** Every command a panel reaches: its buttons, the choices of its split buttons and its ▾ list. */
export function panelCommands(p: RibbonPanel): string[] {
  const ids = p.items.flatMap((i) => (i.kind === 'command' ? [i.id] : i.kind === 'split' ? i.entries.map((e) => e.command) : []));
  return [...new Set([...ids, ...(p.overflow ?? [])])];
}

/** An entry before sizing: families and tools with methods become splits. */
type Pre =
  | { kind: 'command'; id: string }
  | { kind: 'menu'; menu: SubmenuSpec }
  | { kind: 'builtin'; name: BuiltinPanel }
  | { kind: 'split'; key: string; entries: SplitEntry[]; members: ToolDescriptor[] };

/** A tool's choices: its methods, or the tool itself. */
function splitEntries(t: ToolDescriptor): SplitEntry[] {
  const command = `tool.${t.id}`;
  return t.methods?.length
    ? t.methods.map((m) => ({ command, option: m.option, title: t.label, label: m.label, description: m.description, ...(m.aliases?.length && { aliases: m.aliases }), ...(m.icon && { icon: m.icon }) }))
    : [{ command, title: t.label, label: t.label }];
}

/** Family members (at the first one's place) and tools with methods as split buttons. */
function groupSplits(entries: readonly Entry[], toolOf: ReadonlyMap<string, ToolDescriptor>): Pre[] {
  const out: Pre[] = [];
  const families = new Map<string, Extract<Pre, { kind: 'split' }>>();
  for (const e of entries) {
    const t = e.kind === 'command' ? toolOf.get(e.id) : undefined;
    if (!t || (!t.family && !t.methods?.length)) {
      out.push(e);
      continue;
    }
    const key = t.family ?? t.id;
    const found = families.get(key);
    if (found) {
      found.members.push(t);
      found.entries.push(...splitEntries(t));
      continue;
    }
    const split: Extract<Pre, { kind: 'split' }> = { kind: 'split', key, entries: splitEntries(t), members: [t] };
    families.set(key, split);
    out.push(split);
  }
  // A family with one member left in this mode and no methods is a plain button.
  return out.map((x) => (x.kind === 'split' && x.entries.length === 1 ? { kind: 'command', id: x.entries[0].command } : x));
}

/**
 * Menu blocks as ribbon panels: an untitled block takes the menu's name,
 * an inline submenu contributes its own blocks, `@models` lists the model
 * library and `@processing` becomes one panel per category.
 */
function expandBlocks(blocks: readonly MenuBlock[], fallback: string, inputs: RibbonInputs, tools: readonly ToolDescriptor[], filter: WorkspaceFilter): { label: string; entries: Entry[] }[] {
  const out: { label: string; entries: Entry[] }[] = [];
  for (const block of blocks) {
    let cur: { label: string; entries: Entry[] } = { label: block.label || fallback, entries: [] };
    out.push(cur);
    for (const e of block.items) {
      if (typeof e === 'object') {
        if (!e.inline) {
          cur.entries.push({ kind: 'menu', menu: e });
          continue;
        }
        out.push(...expandBlocks(menuBlocks(e.items, tools, filter), e.label, inputs, tools, filter));
      } else if (e === '@models') {
        cur.entries.push({ kind: 'command', id: 'processing.newModel' }, ...inputs.models.map((m) => ({ kind: 'command' as const, id: modelCommandId(m.id) })));
        continue;
      } else if (e === '@processing') {
        for (const node of inputs.processing) out.push({ label: node.category.label, entries: categoryTools(node).map((id) => ({ kind: 'command' as const, id })) });
      } else {
        cur.entries.push({ kind: 'command', id: e });
        continue;
      }
      // After panels of their own, the block's remaining items continue in a new one under its name.
      cur = { label: block.label || fallback, entries: [] };
      out.push(cur);
    }
  }
  return out;
}

/** A category's tools and those of its sub-categories, as commands. */
function categoryTools(node: CategoryNode): string[] {
  return [...node.tools.map((t) => processingCommandId(t.id)), ...node.children.flatMap(categoryTools)];
}
