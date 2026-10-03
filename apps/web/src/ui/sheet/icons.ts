import { ICONS } from '../icons';

/**
 * The sheet layouts' icons (docs/sheet/tasks-web.md A6), drawn by the icon
 * set's rule (DESIGN.md §6): 20×20, `currentColor`, 1.4 px strokes with round
 * ends, no fill but the deliberate 14–35 % ones. They are added to the set
 * when the sheets are installed, never over an icon it has, so ribbon
 * buttons, menus and panels draw them with `icon()` like any other; the
 * existing ones (select, pan, text, legend, table, export …) are used as they are.
 */
export const SHEET_ICONS: Readonly<Record<string, string>> = {
  // The layout itself: a paper with a map frame and a title block under it.
  sheetLayout: '<rect x="3.5" y="2.5" width="13" height="15" rx="1"/><rect x="5.5" y="4.5" width="9" height="6.5"/><path d="M5.5 13h9v2.5h-9zM10.5 13v2.5"/>',
  // Yeni pafta: a paper with a plus at its corner.
  sheetNew: '<path d="M11 2.5H4.5a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1V9"/><path d="M14.5 1.8v5.4M11.8 4.5h5.4"/>',
  // Şablondan: papers in a stack, the top one written on.
  sheetTemplate: '<rect x="6.5" y="2.5" width="10" height="12" rx="1"/><path d="M4 5.5v11a1 1 0 0 0 1 1h8.5"/><path d="M8.7 5.5h5.6M8.7 8h5.6M8.7 10.5h3.4"/>',
  // Şablon olarak kaydet: a paper and a star at its corner.
  sheetSaveTemplate: '<path d="M11.5 2.5h-7a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1v-6.5"/><path d="m14.4 1.8 1 2 2.2.3-1.6 1.6.4 2.2-2-1-2 1 .4-2.2-1.6-1.6 2.2-.3z"/>',
  // Sayfa ayarları: a paper with its width and height marked.
  sheetPage: '<rect x="6" y="5" width="10.5" height="12.5" rx=".6"/><path d="M6 2.5h10.5M3.5 5v12.5" stroke-width="1.1"/><path d="m7.2 1.6-1.2.9 1.2.9M15.3 1.6l1.2.9-1.2.9M2.6 6.2l.9-1.2.9 1.2M2.6 16.3l.9 1.2.9-1.2" stroke-width="1.1"/>',
  // Ön denetim: a clipboard with a tick and a line.
  sheetPreflight: '<path d="M7 3.5H5.5a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1v-12a1 1 0 0 0-1-1H13"/><rect x="7" y="2.3" width="6" height="2.6" rx=".6"/><path d="m7 10.3 1.7 1.7 3.6-3.8M7.5 14.6h5"/>',
  // Sayfayı sığdır and Gerçek boyut.
  sheetZoomPage: '<rect x="6" y="5" width="8" height="10" rx=".5"/><path d="M2.5 6V2.5H6M14 2.5h3.5V6M17.5 14v3.5H14M6 17.5H2.5V14"/>',
  sheetZoomReal: '<path d="M4.3 6.4 6 5v10M13.7 6.4 15.4 5v10"/><path d="M10 8.2v.1M10 11.8v.1" stroke-width="2"/>',
  // Items: the map frame (a view of the drawing in a frame), the scale bar, the KentOS north arrow (half filled, “K” on top).
  sheetMap: '<rect x="2.5" y="3.5" width="15" height="13" rx=".6"/><path d="m2.5 13.3 4-3.4 3 2.1 4.3-4.8 3.7 2.7"/><path d="M9.3 3.5 7.6 9.4" stroke-dasharray="1.6 1.4"/>',
  sheetScaleBar: '<rect x="2.5" y="8.5" width="15" height="3"/><path d="M6.25 8.5v3M10 8.5v3M13.75 8.5v3"/><path d="M2.5 8.5h3.75v3H2.5zM10 8.5h3.75v3H10z" fill="currentColor" fill-opacity=".35" stroke="none"/><path d="M2.5 14v1.6M10 14v1.6M17.5 14v1.6"/>',
  sheetNorth: '<path d="M10 7.2 13.6 17.3 10 15 6.4 17.3z"/><path d="M10 7.2V15l-3.6 2.3z" fill="currentColor" fill-opacity=".35" stroke="none"/><path d="M8.6 1.8v3.8M8.6 3.9l2.6-2.1M9.4 3.3l1.8 2.3"/>',
  // Koordinat listesi: a table of names and two columns of numbers.
  sheetCoordinates: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7h15M7.5 3.5v13M12.5 3.5v13"/><path d="M4 10h2M4 13h2M9 10h2M9 13h2M14 10h2M14 13h2" stroke-width="1.2"/>',
  // Antet: the title block's cells, its title line.
  sheetTitleBlock: '<rect x="2.5" y="5" width="15" height="10.5" rx=".5"/><path d="M2.5 8.8h15M2.5 12.2h15M9.5 8.8v6.7M13.5 12.2v3.3"/><path d="M4.5 6.9h6" stroke-width="1.2"/>',
  sheetPicture: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><circle cx="7" cy="8" r="1.6"/><path d="m2.5 14.5 4.5-4 3 2.5 3-3 4.5 4"/>',
  // Şekil and its kinds; Çizgi with an arrow at its end.
  sheetShape: '<rect x="2.5" y="7.5" width="9" height="9" rx=".5"/><circle cx="12.5" cy="7.5" r="4.6"/>',
  sheetRect: '<rect x="3" y="5" width="14" height="10" rx="1"/>',
  sheetEllipse: '<ellipse cx="10" cy="10" rx="7" ry="5"/>',
  sheetTriangle: '<path d="M10 3.5 17 16H3z"/>',
  sheetPolygon: '<path d="m10 3 6.6 4.8-2.5 7.7H5.9L3.4 7.8z"/>',
  sheetLine: '<path d="M3.5 15.5 16.5 4.5"/><path d="m12.4 4.6 4.1-.1-.1 4.1"/>',
  sheetArrow: '<path d="M3 10h13"/><path d="m12 6 4.5 4-4.5 4"/>',
  // Ready looks of their own (Şekil ▾, Çizgi ▾, Ölçek ▾, Çerçeve ▾, Tablo ▾, Kuzey ▾): a rounded rectangle; a plain
  // line; a numeric scale (1:n); a single heavy frame; a revisions table (its deltas) and a drawing list (its sheets);
  // a compass rose.
  sheetRounded: '<rect x="3" y="5" width="14" height="10" rx="3.2"/>',
  sheetLinePlain: '<path d="M3.5 15.5 16.5 4.5"/>',
  sheetScaleNumeric:
    '<rect x="2.5" y="5" width="15" height="10" rx="1"/><path d="m5.8 8.3 1.7-1.3v6M12.3 13V9.8a1.6 1.6 0 0 1 3.2 0V13" stroke-width="1.2"/><circle cx="10" cy="8.6" r=".9" fill="currentColor" stroke="none"/><circle cx="10" cy="11.6" r=".9" fill="currentColor" stroke="none"/>',
  sheetBorderNeat: '<rect x="2.5" y="3.5" width="15" height="13" rx=".4" stroke-width="2"/>',
  sheetTableRevisions:
    '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.5h15M7.5 7.5v9" stroke-width="1.1"/><path d="m5 9.3 1.4 2.4H3.6zM5 12.8l1.4 2.4H3.6z" stroke-width="1"/><path d="M9.5 10.5h6M9.5 14h4.5" stroke-width="1.1"/>',
  sheetTableDrawings:
    '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.5h15M7.5 7.5v9" stroke-width="1.1"/><rect x="3.9" y="8.9" width="2.2" height="2.8" rx=".3" stroke-width="1"/><rect x="3.9" y="12.6" width="2.2" height="2.8" rx=".3" stroke-width="1"/><path d="M9.5 10.5h6M9.5 14h4.5" stroke-width="1.1"/>',
  // CAD's map preset, the viewport: the model's line with its grips (CBS's is the map); Lejant ▾: the
  // layers with their line types, a thematic legend's graded classes.
  sheetViewport:
    '<rect x="2.5" y="3.5" width="15" height="13" rx=".6"/><path d="m5.5 13 3-4.5 3 2.5 3-4.5" stroke-width="1.1"/><rect x="4" y="11.5" width="3" height="3" fill="currentColor" stroke="none"/><rect x="13" y="5" width="3" height="3" fill="currentColor" stroke="none"/>',
  sheetLegendLayers: '<path d="M3 5h4"/><path d="M3 10h4" stroke-dasharray="1.6 1.4"/><path d="M3 15h4" stroke-dasharray="3 1.3 .1 1.3"/><path d="M9.5 5h7.5M9.5 10h7.5M9.5 15h5"/>',
  sheetLegendThematic:
    '<rect x="3" y="3.5" width="4" height="3" rx=".5" fill="currentColor" fill-opacity=".14"/><rect x="3" y="8.5" width="4" height="3" rx=".5" fill="currentColor" fill-opacity=".45"/><rect x="3" y="13.5" width="4" height="3" rx=".5" fill="currentColor" fill-opacity=".85"/><path d="M9.5 5h7.5M9.5 10h7.5M9.5 15h5"/>',
  sheetCompass:
    '<circle cx="10" cy="10" r="5" stroke-width="1.1"/><path d="M10 2.5 11.5 8.5 17.5 10 11.5 11.5 10 17.5 8.5 11.5 2.5 10 8.5 8.5z"/><path d="M10 2.5 11.5 8.5 10 10 8.5 8.5z" fill="currentColor" fill-opacity=".45" stroke="none"/>',
  // Pafta çerçevesi: two frames, the band between them with its zone ticks.
  sheetBorder: '<rect x="2.5" y="3.5" width="15" height="13" rx=".4"/><rect x="5" y="6" width="10" height="8" rx=".3"/><path d="M10 3.5V6M10 14v2.5M2.5 10H5M15 10h2.5" stroke-width="1.1"/>',
  // Karelaj: the grid's crosses in a frame.
  sheetGrid: '<rect x="2.5" y="2.5" width="15" height="15" rx=".6"/><path d="M7.5 6v3M6 7.5h3M12.5 6v3M11 7.5h3M7.5 11v3M6 12.5h3M12.5 11v3M11 12.5h3" stroke-width="1.2"/>',
  // Atlas: pages in a stack, a map on the top one.
  sheetAtlas: '<rect x="6" y="2.5" width="11.5" height="12.5" rx=".8"/><path d="M4 5v11.5a1 1 0 0 0 1 1h9.5"/><path d="m8 12 2.5-2.5 2 1.5 3-3" stroke-width="1.2"/>',
  // Genel bakış haritası: a map with the main map's frame marked in it.
  sheetOverview: '<rect x="2.5" y="3.5" width="15" height="13" rx=".6"/><rect x="9" y="7" width="5" height="4" fill="currentColor" fill-opacity=".3"/><path d="m2.5 14 4-3.5 3 2"/>',
  // Değişkenler: @ in a box.
  sheetVariables: '<rect x="2.5" y="3.5" width="15" height="13" rx="1.5"/><circle cx="10" cy="10" r="2.2"/><path d="M12.2 10v1.2a1.5 1.5 0 0 0 3 0V10a5.2 5.2 0 1 0-2 4.1"/>',
  // Grupla and Grubu çöz.
  sheetGroup: '<rect x="2.5" y="2.5" width="15" height="15" rx="1" stroke-dasharray="2 1.6"/><rect x="5" y="5" width="5.5" height="5.5" rx=".5"/><rect x="9.5" y="9.5" width="5.5" height="5.5" rx=".5"/>',
  sheetUngroup: '<rect x="2.5" y="2.5" width="6.5" height="6.5" rx=".5"/><rect x="11" y="11" width="6.5" height="6.5" rx=".5"/><path d="M11.5 5.5h3v3M8.5 14.5h-3v-3" stroke-dasharray="1.6 1.4"/>',
  // Hizala: the line they meet on, the items' bars.
  sheetAlignLeft: '<path d="M3.5 2.5v15"/><rect x="5.5" y="5" width="10" height="3.5" rx=".5"/><rect x="5.5" y="11.5" width="6" height="3.5" rx=".5"/>',
  sheetAlignCenter: '<path d="M10 2.5v15"/><rect x="4" y="5" width="12" height="3.5" rx=".5"/><rect x="6.5" y="11.5" width="7" height="3.5" rx=".5"/>',
  sheetAlignRight: '<path d="M16.5 2.5v15"/><rect x="4.5" y="5" width="10" height="3.5" rx=".5"/><rect x="8.5" y="11.5" width="6" height="3.5" rx=".5"/>',
  sheetAlignTop: '<path d="M2.5 3.5h15"/><rect x="5" y="5.5" width="3.5" height="10" rx=".5"/><rect x="11.5" y="5.5" width="3.5" height="6" rx=".5"/>',
  sheetAlignMiddle: '<path d="M2.5 10h15"/><rect x="5" y="4" width="3.5" height="12" rx=".5"/><rect x="11.5" y="6.5" width="3.5" height="7" rx=".5"/>',
  sheetAlignBottom: '<path d="M2.5 16.5h15"/><rect x="5" y="4.5" width="3.5" height="10" rx=".5"/><rect x="11.5" y="8.5" width="3.5" height="6" rx=".5"/>',
  // Hizala's reference: the page (its centre marks), its margins (dashed inside it), the selection's box (dashed).
  sheetAlignToPage:
    '<rect x="3.5" y="2.5" width="13" height="15" rx=".6" stroke-width="1.1"/><rect x="7" y="7.5" width="6" height="5" rx=".5"/><path d="M10 2.5V5M10 15v2.5M3.5 10h2M14.5 10h2" stroke-width="1.1"/>',
  sheetAlignToMargins:
    '<rect x="3.5" y="2.5" width="13" height="15" rx=".6" stroke-width="1.1"/><rect x="5.6" y="4.6" width="8.8" height="10.8" stroke-dasharray="1.4 1.3" stroke-width="1.1"/><rect x="7.6" y="8" width="4.8" height="4" rx=".5"/>',
  sheetAlignToSelection:
    '<rect x="2.5" y="3" width="15" height="14" stroke-dasharray="1.6 1.4" stroke-width="1.1"/><rect x="5" y="5.8" width="8.5" height="3.4" rx=".5"/><rect x="7.5" y="10.8" width="7.5" height="3.4" rx=".5"/>',
  // Dağıt: three items, equal gaps between them.
  sheetDistributeH: '<rect x="2.5" y="6" width="3.5" height="8" rx=".5"/><rect x="8.25" y="6" width="3.5" height="8" rx=".5"/><rect x="14" y="6" width="3.5" height="8" rx=".5"/><path d="M2.5 3h15" stroke-dasharray="1.6 1.4"/>',
  sheetDistributeV: '<rect x="6" y="2.5" width="8" height="3.5" rx=".5"/><rect x="6" y="8.25" width="8" height="3.5" rx=".5"/><rect x="6" y="14" width="8" height="3.5" rx=".5"/><path d="M3 2.5v15" stroke-dasharray="1.6 1.4"/>',
  // Equal gaps (rather than centres): the gaps marked.
  sheetDistributeHGaps:
    '<rect x="2.5" y="4" width="3" height="8.5" rx=".5"/><rect x="8.25" y="4" width="3.5" height="8.5" rx=".5"/><rect x="14.5" y="4" width="3" height="8.5" rx=".5"/><path d="M5.5 15.5h2.75M11.75 15.5h2.75M5.5 14.2v2.6M8.25 14.2v2.6M11.75 14.2v2.6M14.5 14.2v2.6" stroke-width="1.1"/>',
  sheetDistributeVGaps:
    '<rect x="4" y="2.5" width="8.5" height="3" rx=".5"/><rect x="4" y="8.25" width="8.5" height="3.5" rx=".5"/><rect x="4" y="14.5" width="8.5" height="3" rx=".5"/><path d="M15.5 5.5v2.75M15.5 11.75v2.75M14.2 5.5h2.6M14.2 8.25h2.6M14.2 11.75h2.6M14.2 14.5h2.6" stroke-width="1.1"/>',
  // Aynı genişlik and Aynı yükseklik: two items, the size they take marked.
  sheetMatchWidth:
    '<rect x="5" y="7.5" width="10" height="3" rx=".5"/><rect x="5" y="12.5" width="10" height="5" rx=".5"/><path d="M5 4h10M6.8 2.4 5 4l1.8 1.6M13.2 2.4 15 4l-1.8 1.6" stroke-width="1.1"/>',
  sheetMatchHeight:
    '<rect x="7.5" y="5" width="3" height="10" rx=".5"/><rect x="12.5" y="5" width="5" height="10" rx=".5"/><path d="M4 5v10M2.4 6.8 4 5l1.6 1.8M2.4 13.2 4 15l1.6-1.8" stroke-width="1.1"/>',
  // Modele dön: the model's axes and a drawn line with its grips.
  sheetModel:
    '<path d="M3.5 16.5V8M3.5 16.5H12" stroke-width="1.1"/><path d="m2 9.6 1.5-2 1.5 2M10.4 15l2 1.5-2 1.5" stroke-width="1.1"/><path d="m6.8 12.6 3-5.2 3.4 3 3.6-6"/><rect x="5.3" y="11.1" width="3" height="3" fill="currentColor" stroke="none"/><rect x="15.3" y="2.9" width="3" height="3" fill="currentColor" stroke="none"/>',
  // A sheet's tab moved left or right.
  sheetMoveLeft: '<path d="M16 10H4.5M8.5 6l-4 4 4 4"/>',
  sheetMoveRight: '<path d="M4 10h11.5M11.5 6l4 4-4 4"/>',
  // Sıra: to the front and back (the one in front tinted), a step forward and back.
  sheetFront: '<rect x="3" y="3" width="9" height="9" rx=".5"/><rect x="8" y="8" width="9" height="9" rx=".5" fill="currentColor" fill-opacity=".3"/>',
  sheetBack: '<rect x="8" y="8" width="9" height="9" rx=".5" fill="currentColor" fill-opacity=".3" stroke-dasharray="2 1.5"/><rect x="3" y="3" width="9" height="9" rx=".5"/>',
  sheetForward: '<rect x="3.5" y="8.5" width="9" height="8" rx=".5"/><path d="M15 11.5V3.5m-2.5 2.5L15 3.5l2.5 2.5"/>',
  sheetBackward: '<rect x="3.5" y="3.5" width="9" height="8" rx=".5"/><path d="M15 8.5v8m-2.5-2.5L15 16.5l2.5-2.5"/>',
};

/** Adds the sheet icons to the set (never over one it has). */
export function registerSheetIcons(): void {
  const set = ICONS as unknown as Record<string, string>;
  for (const [name, svg] of Object.entries(SHEET_ICONS)) if (!(name in set)) set[name] = svg;
}
