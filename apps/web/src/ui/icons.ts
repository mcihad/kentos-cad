import { ARROW_ICONS } from './arrowIcons';
import { HATCH_ICONS, HATCH_PREVIEWS } from './hatchIcons';
import { SERVICE_ICONS } from './serviceIcons';

/**
 * Hand-drawn 20×20 stroke icon set. Filled squares are CAD grips, so a tool
 * icon shows where its clicks go. Stroke = currentColor, 1.4 px.
 */
const grip = (x: number, y: number) => `<rect x="${x - 1.5}" y="${y - 1.5}" width="3" height="3" fill="currentColor" stroke="none"/>`;
/** A small padlock in a digitizing lock's corner (docs/adr/0166): its body's top left at (x, y), 5.6 × 4.2. */
const padlock = (x: number, y: number) =>
  `<rect x="${x}" y="${y}" width="5.6" height="4.2" rx=".7" fill="currentColor" fill-opacity=".25" stroke-width="1.1"/><path d="M${x + 1.3} ${y}v-1.2a1.5 1.5 0 0 1 3 0V${y}" stroke-width="1.1"/>`;
/**
 * A text's alignment (docs/adr/0145 §6): the text's box, dashed, its baseline and, filled, the point `p` is. Left,
 * centre and right are x 3, 10 and 17; top, middle, baseline and bottom y 4.5, 8.5, 12.5 and 15.5.
 */
const textAlign = (x: number, y: number) =>
  `<rect x="3" y="4.5" width="14" height="11" rx="1" stroke-dasharray="2 1.6"/><path d="M3 12.5h14" stroke-width="1"/><circle cx="${x}" cy="${y}" r="2.6" fill="currentColor" stroke="none"/>`;

/**
 * A file kind brought in or written out: the page, the arrow into or out of it on the left, the kind's emblem on it
 * (DXF a line between grips, NCZ an N, Shapefile a polygon, GeoJSON braces, a coordinate list its rows, PDF a P,
 * SVG a curve with its handle, PNG a picture, .kpafta a sheet with its title block, a GNSS file a position's pin).
 */
const filePage = '<path d="M7.5 2.5H14l3.5 3.5v11.5h-10z" stroke-width="1.2"/><path d="M14 2.5V6h3.5" stroke-width="1.1"/>';
const fileIn = (emblem: string) => `${filePage}<path d="M1.5 11.5h5.5M4.5 9 7 11.5 4.5 14"/>${emblem}`;
const fileOut = (emblem: string) => `${filePage}<path d="M7 11.5H1.5M4 9l-2.5 2.5L4 14"/>${emblem}`;
const dot = (x: number, y: number, r: number) => `<circle cx="${x}" cy="${y}" r="${r}" fill="currentColor" stroke="none"/>`;
const EMBLEM = {
  dxf: `<path d="m9.5 14.5 5.75-5.25" stroke-width="1.1"/>${grip(9.5, 14.5)}${grip(15.25, 9.25)}`,
  ncz: '<path d="M10 15.2V8.8l5 6.4V8.8" stroke-width="1.2"/>',
  shp: '<path d="m12.5 8.4 3.2 2.3-1.2 3.9h-4l-1.2-3.9z" fill="currentColor" fill-opacity=".3" stroke-width="1.1"/>',
  geojson:
    '<path d="M11.4 8.6h-.5a.9.9 0 0 0-.9.9v1.4l-.8.8.8.8v1.4a.9.9 0 0 0 .9.9h.5M14.1 8.6h.5a.9.9 0 0 1 .9.9v1.4l.8.8-.8.8v1.4a.9.9 0 0 1-.9.9h-.5" stroke-width="1.1"/>',
  ncn: `<path d="M11.8 9.4h3.7M11.8 12h3.7M11.8 14.6h3.7" stroke-width="1.1"/>${dot(9.8, 9.4, 0.8)}${dot(9.8, 12, 0.8)}${dot(9.8, 14.6, 0.8)}`,
  pdf: '<path d="M10.6 15.4V8.8h2.5a1.8 1.8 0 0 1 0 3.6h-2.5" stroke-width="1.3"/>',
  svg: `<path d="M9.5 15C10 11 12 9.5 15.5 9.5" stroke-width="1.1"/><path d="M12.5 9.5h3" stroke-width="1"/>${grip(9.5, 15)}${dot(15.5, 9.5, 0.9)}`,
  png: `<path d="m9.4 15.2 2.4-3.2 1.7 2 1.1-1.3 1.4 2.5z" fill="currentColor" fill-opacity=".3" stroke-width="1"/>${dot(14.3, 9.6, 0.9)}`,
  kpafta: '<rect x="9.4" y="8.6" width="6.4" height="7" stroke-width="1"/><path d="M9.4 13.4h6.4M13 13.4v2.2" stroke-width="1"/>',
  gnss: `<path d="M12.7 15.8c-1.9-2.1-2.9-3.6-2.9-4.8a2.9 2.9 0 0 1 5.8 0c0 1.2-1 2.7-2.9 4.8z" stroke-width="1.1"/>${dot(12.7, 11, 0.9)}`,
};

// Exported for the feature inventory: the desktop draws the same icons from it (docs/adr/0054).
export const ICONS = {
  // Tools
  select: '<path d="M5 3.2 15 9.3l-4.4 1.2-2.3 4.3z"/>',
  selectFence: `<path d="M6 3.5v6M14 10.5v6"/><path d="M2.5 11 9 5.5l4 7 4.5-4" stroke-dasharray="2.2 1.5"/>${grip(2.5, 11)}${grip(17.5, 8.5)}`,
  selectCircle: '<circle cx="10" cy="10" r="7.2" stroke-dasharray="2.4 1.6"/><path d="M7 12.6h6L10 7z"/>',
  selectContaining: '<rect x="2.5" y="2.5" width="15" height="15"/><rect x="6" y="6" width="8" height="8" stroke-width="2"/><circle cx="10" cy="10" r="1.1" fill="currentColor" stroke="none"/>',
  // Seçim ekleri (docs/adr/0187), the owner's choices (6 October): a dashed polygon with an object inside and grips at
  // its corners; a gripped example and its dashed like; a dashed box and an undo arrow; a funnel; two stacked boxes and
  // a turning arrow.
  selectPolygon: `<path d="M3 7 9 3l8 3.5-2.5 5L17 16.5 6.5 17z" stroke-dasharray="2.2 1.5"/><path d="M7.8 13.6h4.8L10.2 9.4z"/>${grip(3, 7)}${grip(17, 6.5)}${grip(6.5, 17)}`,
  selectSimilar: `<path d="M2.5 15.5h6.5L5.75 8.5z"/>${grip(2.5, 15.5)}${grip(9, 15.5)}${grip(5.75, 8.5)}<path d="M11 15.5h6.5l-3.25-7z" stroke-dasharray="2.2 1.5"/>`,
  selectPrevious: '<rect x="2.5" y="8.5" width="9" height="9" stroke-dasharray="2 1.6"/><path d="M8.5 5.5h6a3 3 0 0 1 0 6H14"/><path d="M10.5 3 8 5.5 10.5 8"/>',
  selectFilter: '<path d="M3 4h14l-5.5 6.5V16l-3-1.5v-4z"/>',
  selectCycle: '<rect x="2.5" y="6.5" width="8" height="8" stroke-dasharray="2 1.6"/><rect x="6.5" y="2.5" width="8" height="8"/><path d="M17.5 9.5a5 5 0 0 1-6.5 7.3"/><path d="M10.6 14.6l.4 2.2 2.2-.6"/>',
  pan: '<path d="M7.2 10V4.7a1.2 1.2 0 0 1 2.4 0V9M9.6 8.6V3.6a1.2 1.2 0 0 1 2.4 0V9M12 9V4.8a1.2 1.2 0 0 1 2.4 0V11c0 3.4-2 6-5.3 6-2.2 0-3.5-1.2-4.6-3.1l-1.6-2.8a1.25 1.25 0 0 1 2.1-1.4L7.2 12"/>',
  point: '<path d="M10 3.5v13M3.5 10h13"/><circle cx="10" cy="10" r="3.2"/>',
  line: `<path d="M4.5 15.5 15.5 4.5"/>${grip(4.5, 15.5)}${grip(15.5, 4.5)}`,
  polyline: `<path d="m3.5 15 4-8 5 5 4-8"/>${grip(3.5, 15)}${grip(7.5, 7)}${grip(12.5, 12)}${grip(16.5, 4)}`,
  arc: `<path d="M3.5 16A12.5 12.5 0 0 1 16 3.5"/>${grip(3.5, 16)}${grip(16, 3.5)}${grip(7.2, 7.2)}`,
  // The arc from its centre (the dot) and ends; and going on from the last line's end, tangent to it.
  arcCenter: `<path d="M16 15A11 11 0 0 0 5 4"/><path d="M5 15h11M5 15V4" stroke-dasharray="1.4 1.6" stroke-width="1.1"/><circle cx="5" cy="15" r="1.4" fill="currentColor" stroke="none"/>${grip(16, 15)}${grip(5, 4)}`,
  arcContinue: `<path d="M2.5 15.5H8" stroke-width="1.1"/><path d="M8 15.5a7.5 7.5 0 0 0 7.5-7.5V5"/><path d="m13.2 6.8 2.3-2.3 2.3 2.3"/>${grip(8, 15.5)}`,
  circle: `<circle cx="10" cy="10" r="6.5"/><path d="M10 10h6.5"/>${grip(10, 10)}`,
  // The circle's other ways: a diameter's ends; three points on it; tangent to two lines with its radius; inside three.
  circle2p: `<circle cx="10" cy="10" r="6.5"/><path d="M3.5 10h13" stroke-width="1.1"/>${grip(3.5, 10)}${grip(16.5, 10)}`,
  circle3p: `<circle cx="10" cy="10" r="6.5"/>${grip(10, 3.5)}${grip(4.37, 13.25)}${grip(15.63, 13.25)}`,
  circleTtr: '<path d="M3 2.5V17h14.5" stroke-width="1.1"/><circle cx="8.5" cy="11.5" r="5.5"/><path d="m8.5 11.5 3.9-3.9"/><circle cx="8.5" cy="11.5" r="1" fill="currentColor" stroke="none"/>',
  circleTtt: '<path d="M2.5 17h15L10 3z" stroke-width="1.1"/><circle cx="10" cy="12.51" r="4.49"/>',
  rectangle: `<rect x="3.5" y="5.5" width="13" height="9"/>${grip(3.5, 14.5)}${grip(16.5, 5.5)}`,
  ellipse: `<ellipse cx="10" cy="10" rx="7.6" ry="4.3" transform="rotate(-28 10 10)"/>${grip(3.3, 13.6)}${grip(16.7, 6.4)}`,
  xline: `<path d="M1.5 16 18.5 4" stroke-dasharray="3.2 1.6"/>${grip(10, 10)}`,
  ray: `<path d="M3.5 15.5 18 5.3" stroke-dasharray="3.2 1.6"/><path d="m14.4 5 3.6.3-1.4 3.3"/>${grip(3.5, 15.5)}`,
  rectangle3: `<path d="M3 12.5 11 4l6.5 6.5-8 8.5z"/>${grip(3, 12.5)}${grip(11, 4)}${grip(9.5, 19)}`,
  regularPolygon: `<path d="M10 2.8 16.3 6.4v7.2L10 17.2l-6.3-3.6V6.4z"/><circle cx="10" cy="10" r=".9" fill="currentColor"/>${grip(10, 2.8)}`,
  polygon: `<path d="m3.8 8 6-4.5 6.5 3.5-1.8 9H5.6z" fill="currentColor" fill-opacity=".14"/>${grip(3.8, 8)}${grip(9.8, 3.5)}${grip(16.3, 7)}${grip(14.5, 16)}${grip(5.6, 16)}`,
  // Bitişik alan: a neighbour on the left, the new area against it, only its new boundary drawn (docs/adr/0162 §3).
  adjoin: `<path d="M2.5 4.5h7v11h-7z"/><path d="M9.5 6.5 14 3.5l3.5 5-1.5 7H9.5z" fill="currentColor" fill-opacity=".16" stroke="none"/><path d="M9.5 6.5 14 3.5l3.5 5-1.5 7H9.5"/>${grip(9.5, 6.5)}${grip(9.5, 15.5)}`,
  spline: `<path d="M3 15c2.6-8.5 5.8-8.5 7 0 1.2 8.2 4.5 4 7-9"/>${grip(3, 15)}${grip(17, 6)}`,
  text: '<path d="M4.5 5V3.8h11V5M10 3.8v12.4M7.5 16.2h5"/>',
  // Çok satırlı yazı (docs/adr/0182 §4): a small T at the box's corner, the paragraph's lines wrapped in the box.
  mtext: '<path d="M2.8 3.2h5.4M5.5 3.2v5.3"/><path d="M10.5 4.2h6.7M10.5 7.4h6.7M2.8 11h14.4M2.8 14.2h14.4M2.8 17.4h9" stroke-width="1.2"/>',
  // The paragraph editor's formats (docs/adr/0182 §4): bold, italic and underlined letters, raised and lowered twos,
  // the colour bar under an A, and a sign (Ω) to put at the cursor.
  textBold: '<path d="M6 3.5h4.6a3 3 0 0 1 0 6H6zM6 9.5h5.4a3.5 3.5 0 0 1 0 7H6z" stroke-width="2"/>',
  textItalic: '<path d="M8.5 3.5h7M4.5 16.5h7M12.2 3.5 7.8 16.5"/>',
  textUnderline: '<path d="M6 3.2v6a4 4 0 0 0 8 0v-6"/><path d="M4.5 17.3h11"/>',
  textSuperscript: '<path d="m2.8 8.2 7 8.6M9.8 8.2l-7 8.6"/><path d="M12.4 4.6a1.6 1.6 0 1 1 2.9 1l-2.9 3.3h3.5" stroke-width="1.1"/>',
  textSubscript: '<path d="m2.8 3.4 7 8.6M9.8 3.4l-7 8.6"/><path d="M12.4 12.2a1.6 1.6 0 1 1 2.9 1l-2.9 3.3h3.5" stroke-width="1.1"/>',
  textColor: '<path d="M5.5 13 10 3l4.5 10M7.3 9.2h5.4"/><rect x="3" y="15" width="14" height="2.6" rx=".6" fill="currentColor" stroke="none"/>',
  textSymbol: '<path d="M3.8 16.5h3.9v-1.7a6 6 0 1 1 4.6 0v1.7h3.9"/>',
  // Metin dosyası yerleştir (docs/adr/0145 §6): a page with its corner turned, a T on it.
  textFile: '<path d="M5 2.5h7l3 3v12H5z"/><path d="M12 2.5v3h3"/><path d="M7.5 9.5h5M10 9.5v5"/>',
  // Çizimler arası alışveriş (docs/adr/0193): the selection written out (a dashed box round a gripped object on the
  // page), another drawing's layers brought in, a drawing file placed as a block (Blok ekle's square a page).
  saveSelection: fileOut(`<rect x="9.6" y="8.6" width="6" height="6.2" stroke-dasharray="1.6 1.2" stroke-width="1"/><path d="m11 13.4 3.2-3.4" stroke-width="1.1"/>${grip(11, 13.4)}`),
  takeFrom: fileIn('<path d="m12.6 8.6 3 1.6-3 1.6-3-1.6z" stroke-width="1"/><path d="m9.6 12.6 3 1.6 3-1.6" stroke-width="1"/>'),
  blockInsertFile: `<path d="M8.5 2.5h5.5l2.5 2.5v7.5h-8z"/><path d="M14 2.5V5h2.5"/><circle cx="12.5" cy="8.6" r="1.8"/><path d="M3.5 16.5 8.5 11.5"/>${grip(3.5, 16.5)}`,
  // Resim ekle (docs/adr/0192 §5): a picture, its mountain and sun, the two clicks at its lower corners.
  imageInsert: `<rect x="3" y="4" width="14" height="11" rx=".8"/><path d="m5.6 12.6 3-3.4 2.3 2.2 1.6-1.6 2.1 2.8" stroke-width="1.2"/>${dot(13.2, 7.2, 1.1)}${grip(3, 15)}${grip(17, 15)}`,
  // Raster katmanları (docs/adr/0204; the owner's choices, 8 October): Raster ekle a pixel grid with toned cells and
  // a plus; Raster stili a terrain's profile over a colour ramp; Raster oturt a tilted grid tied by a dashed link from
  // its corner to a control point.
  rasterAdd: '<rect x="2.5" y="3" width="11.5" height="11.5" rx="1"/><path d="M6.33 3v11.5M10.17 3v11.5M2.5 6.83h11.5M2.5 10.67h11.5" stroke-width="1"/><rect x="2.5" y="3" width="3.83" height="3.83" fill="currentColor" fill-opacity=".55" stroke="none"/><rect x="6.33" y="6.83" width="3.84" height="3.84" fill="currentColor" fill-opacity=".55" stroke="none"/><rect x="10.17" y="10.67" width="3.83" height="3.83" fill="currentColor" fill-opacity=".28" stroke="none"/><rect x="6.33" y="3" width="3.84" height="3.83" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M16 12.5v6M13 15.5h6" stroke-width="1.6"/>',
  rasterStyle: '<path d="M2.5 12 6.5 6l3 3.5L12.5 5l5 7" stroke-width="1.3"/><path d="M2.5 12 6.5 6l3 3.5L12.5 5l5 7z" fill="currentColor" fill-opacity=".22" stroke="none"/><rect x="2.5" y="14.5" width="15" height="3" rx=".6" stroke-width="1.1"/><rect x="6.25" y="14.5" width="3.75" height="3" fill="currentColor" fill-opacity=".3" stroke="none"/><rect x="10" y="14.5" width="3.75" height="3" fill="currentColor" fill-opacity=".6" stroke="none"/><rect x="13.75" y="14.5" width="3.75" height="3" fill="currentColor" fill-opacity=".9" stroke="none"/>',
  rasterGeoref: '<path d="M2.5 7 11 4.5l1.5 9-8.5 2.5z" stroke-width="1.3"/><path d="M6.8 5.8l1.4 8.6M3.2 11.4l8.6-2.4" stroke-width=".9"/><path d="M11 4.5l3.6-1.1" stroke-width="1" stroke-dasharray="1.4 1.2"/><circle cx="16.5" cy="3" r="1.6" stroke-width="1.1"/><path d="M16.5 .6v1.2M16.5 4.2v1.2M14.1 3h1.2M17.7 3h1.2" stroke-width="1"/><path d="M4 16l-1.6 1.6" stroke-width="1" stroke-dasharray="1.4 1.2"/><circle cx="2" cy="18" r="1.3" fill="currentColor" stroke="none"/>',
  // Nokta bulutu (docs/adr/0207; the owner's choices, 8 October, every option sheet's A): Nokta bulutu ekle a mound of
  // dots and a plus; Nokta bulutu stili dots in three tones over a colour ramp; XYZ sor a dot ringed and crossed; Sanal
  // bulut olarak kaydet four dotted tiles in a dashed frame; İşlemler's tools: an area's dots beside bars, dense dots to
  // sparse ones, ground dots filled under hollow ones, a tree of dots in three tones, crop marks round the dots kept, two
  // clusters into one, 2 × 2 tiles over dots, dots into toned cells, a right-angled boundary round dots.
  pointCloudAdd:
    '<circle cx="3" cy="13.5" r="0.95" fill="currentColor" stroke="none"/><circle cx="5.2" cy="11.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="5.6" cy="14.6" r="0.95" fill="currentColor" stroke="none"/><circle cx="7.6" cy="9" r="0.95" fill="currentColor" stroke="none"/><circle cx="8.2" cy="12.6" r="0.95" fill="currentColor" stroke="none"/><circle cx="10.2" cy="7.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="10.8" cy="10.8" r="0.95" fill="currentColor" stroke="none"/><circle cx="12.8" cy="8.6" r="0.95" fill="currentColor" stroke="none"/><circle cx="13.6" cy="5.6" r="0.95" fill="currentColor" stroke="none"/><path d="M16 12.5v6M13 15.5h6" stroke-width="1.6"/>',
  pointCloudStyle:
    '<circle cx="3.5" cy="10.5" r="0.95" fill="currentColor" fill-opacity="0.35" stroke="none"/><circle cx="6" cy="8.5" r="0.95" fill="currentColor" fill-opacity="0.35" stroke="none"/><circle cx="8.5" cy="6.5" r="0.95" fill="currentColor" fill-opacity="0.65" stroke="none"/><circle cx="11" cy="5" r="0.95" fill="currentColor" fill-opacity="0.65" stroke="none"/><circle cx="13.5" cy="6.5" r="0.95" fill="currentColor" fill-opacity="1" stroke="none"/><circle cx="16" cy="9" r="0.95" fill="currentColor" fill-opacity="1" stroke="none"/><circle cx="5.2" cy="11.6" r="0.95" fill="currentColor" fill-opacity="0.35" stroke="none"/><circle cx="10" cy="9" r="0.95" fill="currentColor" fill-opacity="0.65" stroke="none"/><circle cx="14.6" cy="10.4" r="0.95" fill="currentColor" fill-opacity="1" stroke="none"/><rect x="2.5" y="14.5" width="15" height="3" rx=".6" stroke-width="1.1"/><rect x="6.25" y="14.5" width="3.75" height="3" fill="currentColor" fill-opacity=".3" stroke="none"/><rect x="10" y="14.5" width="3.75" height="3" fill="currentColor" fill-opacity=".6" stroke="none"/><rect x="13.75" y="14.5" width="3.75" height="3" fill="currentColor" fill-opacity=".9" stroke="none"/>',
  pointCloudQuery:
    '<circle cx="3" cy="14" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="5.4" cy="11.4" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="6" cy="15.4" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="13.4" cy="15" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="15.6" cy="12.4" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="16.6" cy="16" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="4" cy="5.6" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="16.4" cy="4.6" r="0.95" fill="currentColor" fill-opacity="0.45" stroke="none"/><circle cx="10" cy="9.4" r="3.4" stroke-width="1.2"/><path d="M10 3.6v2.4M10 12.8v2.4M4.2 9.4h2.4M13.4 9.4h2.4" stroke-width="1.2"/><circle cx="10" cy="9.4" r="1.1" fill="currentColor" stroke="none"/>',
  pointCloudVpc:
    '<rect x="2" y="2" width="16" height="16" rx="1" stroke-width="1" stroke-dasharray="1.8 1.4"/><rect x="4" y="4" width="5.4" height="5.4" rx=".5" stroke-width="1.1"/><rect x="10.6" y="4" width="5.4" height="5.4" rx=".5" stroke-width="1.1"/><rect x="4" y="10.6" width="5.4" height="5.4" rx=".5" stroke-width="1.1"/><rect x="10.6" y="10.6" width="5.4" height="5.4" rx=".5" stroke-width="1.1"/><circle cx="5.8" cy="6.2" r="0.75" fill="currentColor" stroke="none"/><circle cx="7.6" cy="7.4" r="0.75" fill="currentColor" stroke="none"/><circle cx="12.4" cy="7.2" r="0.75" fill="currentColor" stroke="none"/><circle cx="14.2" cy="5.8" r="0.75" fill="currentColor" stroke="none"/><circle cx="6.4" cy="13.6" r="0.75" fill="currentColor" stroke="none"/><circle cx="12.8" cy="12.4" r="0.75" fill="currentColor" stroke="none"/><circle cx="14.2" cy="14.2" r="0.75" fill="currentColor" stroke="none"/>',
  pointCloudArea:
    '<path d="M2.5 6.5 8.5 3l4 3.2-1.4 9.3H3.6z" stroke-width="1.3"/><circle cx="5.2" cy="8.4" r="0.85" fill="currentColor" stroke="none"/><circle cx="8.2" cy="6.6" r="0.85" fill="currentColor" stroke="none"/><circle cx="6.6" cy="11.6" r="0.85" fill="currentColor" stroke="none"/><circle cx="9.4" cy="10.2" r="0.85" fill="currentColor" stroke="none"/><circle cx="5.6" cy="13.6" r="0.85" fill="currentColor" stroke="none"/><path d="M14.2 17v-4M16.6 17v-7.2M19 17v-5.4" stroke-width="1.8"/>',
  pointCloudThin:
    '<circle cx="2.8" cy="6" r="0.8" fill="currentColor" stroke="none"/><circle cx="4.6" cy="5" r="0.8" fill="currentColor" stroke="none"/><circle cx="3.6" cy="8" r="0.8" fill="currentColor" stroke="none"/><circle cx="5.6" cy="7.4" r="0.8" fill="currentColor" stroke="none"/><circle cx="2.6" cy="10.2" r="0.8" fill="currentColor" stroke="none"/><circle cx="4.6" cy="10.6" r="0.8" fill="currentColor" stroke="none"/><circle cx="6.2" cy="9.6" r="0.8" fill="currentColor" stroke="none"/><circle cx="3.4" cy="12.8" r="0.8" fill="currentColor" stroke="none"/><circle cx="5.4" cy="13.2" r="0.8" fill="currentColor" stroke="none"/><path d="M8 9.5h3.4M9.9 7.8l1.6 1.7-1.6 1.7" stroke-width="1.2"/><circle cx="14.6" cy="5.4" r="0.95" fill="currentColor" stroke="none"/><circle cx="17.2" cy="9.4" r="0.95" fill="currentColor" stroke="none"/><circle cx="14" cy="13.2" r="0.95" fill="currentColor" stroke="none"/>',
  pointCloudGround:
    '<path d="M2 15.5 6 14l4 .8 4-1.3 4 .9" stroke-width="1.1" stroke-dasharray="1.6 1.2"/><circle cx="2.8" cy="15.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="5.6" cy="14.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="8.4" cy="14.6" r="0.95" fill="currentColor" stroke="none"/><circle cx="11.2" cy="14.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="14" cy="13.6" r="0.95" fill="currentColor" stroke="none"/><circle cx="16.8" cy="14.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="6.2" cy="6.4" r="1.05" stroke-width="1"/><circle cx="8.6" cy="4.4" r="1.05" stroke-width="1"/><circle cx="10.8" cy="6.8" r="1.05" stroke-width="1"/><circle cx="8.4" cy="9" r="1.05" stroke-width="1"/><circle cx="13.8" cy="8.6" r="1.05" stroke-width="1"/><circle cx="15.6" cy="10.6" r="1.05" stroke-width="1"/>',
  pointCloudClassify:
    '<path d="M2 17h16" stroke-width="1.1"/><circle cx="6" cy="15.2" r="0.95" fill="currentColor" fill-opacity="0.3" stroke="none"/><circle cx="10" cy="15.4" r="0.95" fill="currentColor" fill-opacity="0.3" stroke="none"/><circle cx="14" cy="15.2" r="0.95" fill="currentColor" fill-opacity="0.3" stroke="none"/><circle cx="7.6" cy="11.6" r="0.95" fill="currentColor" fill-opacity="0.6" stroke="none"/><circle cx="10" cy="11" r="0.95" fill="currentColor" fill-opacity="0.6" stroke="none"/><circle cx="12.4" cy="11.6" r="0.95" fill="currentColor" fill-opacity="0.6" stroke="none"/><circle cx="8.6" cy="7.4" r="0.95" fill="currentColor" fill-opacity="1" stroke="none"/><circle cx="11.4" cy="7.4" r="0.95" fill="currentColor" fill-opacity="1" stroke="none"/><circle cx="10" cy="4.2" r="0.95" fill="currentColor" fill-opacity="1" stroke="none"/>',
  pointCloudClip:
    '<path d="M5.5 2.5v3h-3M14.5 2.5v3h3M5.5 17.5v-3h-3M14.5 17.5v-3h3" stroke-width="1.3"/><circle cx="7.6" cy="7.6" r="0.95" fill="currentColor" stroke="none"/><circle cx="10.4" cy="7" r="0.95" fill="currentColor" stroke="none"/><circle cx="12.6" cy="9.4" r="0.95" fill="currentColor" stroke="none"/><circle cx="8.4" cy="11" r="0.95" fill="currentColor" stroke="none"/><circle cx="11" cy="12.6" r="0.95" fill="currentColor" stroke="none"/><circle cx="2.6" cy="9" r="0.8" fill="currentColor" fill-opacity="0.35" stroke="none"/><circle cx="17.4" cy="10.6" r="0.8" fill="currentColor" fill-opacity="0.35" stroke="none"/><circle cx="9.6" cy="16.6" r="0.8" fill="currentColor" fill-opacity="0.35" stroke="none"/><circle cx="8.6" cy="3.2" r="0.8" fill="currentColor" fill-opacity="0.35" stroke="none"/>',
  pointCloudMerge:
    '<circle cx="2.6" cy="3.4" r="0.8" fill="currentColor" stroke="none"/><circle cx="4.8" cy="2.8" r="0.8" fill="currentColor" stroke="none"/><circle cx="3.6" cy="5.4" r="0.8" fill="currentColor" stroke="none"/><circle cx="2.6" cy="14.6" r="0.8" fill="currentColor" stroke="none"/><circle cx="4.8" cy="15.4" r="0.8" fill="currentColor" stroke="none"/><circle cx="3.6" cy="17" r="0.8" fill="currentColor" stroke="none"/><path d="M6.4 5 9.6 8.6M6.4 15 9.6 11.4" stroke-width="1.2"/><circle cx="12" cy="8.4" r="0.95" fill="currentColor" stroke="none"/><circle cx="14.6" cy="7.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="16.8" cy="9.2" r="0.95" fill="currentColor" stroke="none"/><circle cx="12.6" cy="11.4" r="0.95" fill="currentColor" stroke="none"/><circle cx="15.4" cy="11.8" r="0.95" fill="currentColor" stroke="none"/><circle cx="17.6" cy="12.6" r="0.95" fill="currentColor" stroke="none"/>',
  pointCloudTile:
    '<path d="M2.5 2.5h15v15h-15zM10 2.5v15M2.5 10h15" stroke-width="1.1"/><circle cx="4.6" cy="5" r="0.8" fill="currentColor" stroke="none"/><circle cx="7.4" cy="6.4" r="0.8" fill="currentColor" stroke="none"/><circle cx="12.6" cy="4.8" r="0.8" fill="currentColor" stroke="none"/><circle cx="15.2" cy="7" r="0.8" fill="currentColor" stroke="none"/><circle cx="5.2" cy="13.4" r="0.8" fill="currentColor" stroke="none"/><circle cx="7.6" cy="15.2" r="0.8" fill="currentColor" stroke="none"/><circle cx="12.4" cy="13" r="0.8" fill="currentColor" stroke="none"/><circle cx="15.4" cy="15" r="0.8" fill="currentColor" stroke="none"/>',
  pointCloudRaster:
    '<circle cx="2.6" cy="5" r="0.8" fill="currentColor" stroke="none"/><circle cx="4.8" cy="7.2" r="0.8" fill="currentColor" stroke="none"/><circle cx="3" cy="10.4" r="0.8" fill="currentColor" stroke="none"/><circle cx="5.4" cy="12.6" r="0.8" fill="currentColor" stroke="none"/><circle cx="2.8" cy="15.4" r="0.8" fill="currentColor" stroke="none"/><path d="M6.8 10h2.6M8.4 8.6 9.8 10l-1.4 1.4" stroke-width="1.1"/><rect x="11" y="3" width="7" height="14" rx=".5" stroke-width="1.1"/><path d="M11 7.7h7M11 12.3h7M14.5 3v14" stroke-width=".9"/><rect x="11" y="3" width="3.5" height="4.7" fill="currentColor" fill-opacity=".85" stroke="none"/><rect x="14.5" y="7.7" width="3.5" height="4.6" fill="currentColor" fill-opacity=".55" stroke="none"/><rect x="11" y="12.3" width="3.5" height="4.7" fill="currentColor" fill-opacity=".3" stroke="none"/>',
  pointCloudBoundary:
    '<path d="M2.5 6.5h4v-4h8v6h3v9h-9v-4h-6z" stroke-width="1.3"/><circle cx="4.4" cy="8.6" r="0.85" fill="currentColor" stroke="none"/><circle cx="6.6" cy="10.6" r="0.85" fill="currentColor" stroke="none"/><circle cx="8.8" cy="4.6" r="0.85" fill="currentColor" stroke="none"/><circle cx="11" cy="7" r="0.85" fill="currentColor" stroke="none"/><circle cx="12.6" cy="4.8" r="0.85" fill="currentColor" stroke="none"/><circle cx="10" cy="10.2" r="0.85" fill="currentColor" stroke="none"/><circle cx="13" cy="12" r="0.85" fill="currentColor" stroke="none"/><circle cx="15.6" cy="10.6" r="0.85" fill="currentColor" stroke="none"/><circle cx="11.2" cy="15.2" r="0.85" fill="currentColor" stroke="none"/><circle cx="15.2" cy="15.4" r="0.85" fill="currentColor" stroke="none"/>',
  // Resmi kırp: the crop marks around a picture's mountain.
  imageClip: '<path d="M5.5 2.5v12h12"/><path d="M2.5 5.5h12v12"/><path d="m7.4 12 2.1-2.5 1.5 1.5 1.1-1.1 1.4 2.1" stroke-width="1.1"/>',
  // Etiketleri yazıya çevir (docs/adr/0175): a label's tag becoming a text.
  labelsToText: '<path d="M2.5 3.5h6.4l2.6 2.7-2.6 2.7H2.5z"/><circle cx="4.7" cy="6.2" r=".9" fill="currentColor" stroke="none"/><path d="M6 10.5v4h4.3m-1.8-1.8 1.8 1.8-1.8 1.8"/><path d="M12.3 10.8h5.2M14.9 10.8v6.7"/>',
  // Kılavuz (docs/adr/0146): the arrow at the tip, the line, the landing and the note's lines.
  // Koordinat yaz (docs/adr/0185): the place (a grip), the slanted leader and the bar, a Y over the bar and an X under it,
  // each with its value (the owner's choice, 6 October).
  coordinateLabel: `${grip(3.5, 16.5)}<path d="M3.5 16.5 8.5 11.2H18.8"/><path d="M10.2 4.9l1.35 1.7 1.35-1.7M11.55 6.6v2" stroke-width="1.2"/><path d="M14.4 7.9h4" stroke-width="1.5"/><path d="M10.3 12.9l2.5 3M12.8 12.9l-2.5 3" stroke-width="1.2"/><path d="M14.4 14.6h3.4" stroke-width="1.5"/>`,
  // Köşelere koordinat yaz: a parcel, a grip at each corner, two corners' leaders and bars with their values.
  coordinateVertices: `<path d="M2.5 7.5h7v7h-7z" fill="currentColor" fill-opacity=".14" stroke-width="1.2"/>${grip(2.5, 7.5)}${grip(9.5, 7.5)}${grip(2.5, 14.5)}${grip(9.5, 14.5)}<path d="m9.5 7.5 2.5-2.5h6.5M9.5 14.5l2.5 2.5h6.5" stroke-width="1.3"/><path d="M13 2.9h4.6M13 14.9h4.6" stroke-width="1.4"/>`,
  // Km yaz (docs/adr/0189): option B of its sheet, the owner's to change.
  stationLabels: '<path d="M2 15c4-1 6-6 16-7"/><path d="M5.3 12.3l1.4 2.6M10 9.2l1.8 2.2M14.6 7.6l.9 2.6"/><path d="M5 3.5h5M7.5 1v5"/>',
  // Paralel kaydır (docs/adr/0191): a trapeze's top edge moved up, the old one dashed, its sides longer (option B).
  edgeShift: '<path d="M2.5 17.5h15L14 3H6z"/><path d="M4.7 8.5h10.6" stroke-dasharray="2 1.6"/><path d="M10 7.6V4.4m-1.4 1.4L10 4.4l1.4 1.4"/>',
  // Orta hat (docs/adr/0190): two banks and the dash-dot axis between them (option D).
  centerline: '<path d="M2 4.5c3-1.5 6 1.5 9 0s5-1 7 0M2 15.5c3 1.5 6-1.5 9 0s5 1 7 0"/><path d="M2 10h16" stroke-dasharray="4 1.5 1 1.5"/>',
  // Koordinat yaz's Yön (docs/adr/0185 §4): Otomatik, out of an object every way; the four corners, an arrow from the place.
  labelAuto:
    '<path d="M7.5 7.5h5v5h-5z" fill="currentColor" fill-opacity=".14" stroke-width="1.2"/><path d="m12.5 7.5 3.5-3.5M7.5 7.5 4 4M7.5 12.5 4 16M12.5 12.5l3.5 3.5" stroke-width="1.3"/><path d="M13.4 4h2.6v2.6M4 6.6V4h2.6M4 13.4V16h2.6M13.4 16H16v-2.6" stroke-width="1.2"/>',
  labelNorthEast: `${dot(5, 15, 1.5)}<path d="M6.5 13.5 15.5 4.5"/><path d="M10.5 4.5h5v5"/>`,
  labelNorthWest: `${dot(15, 15, 1.5)}<path d="M13.5 13.5 4.5 4.5"/><path d="M9.5 4.5h-5v5"/>`,
  labelSouthWest: `${dot(15, 5, 1.5)}<path d="M13.5 6.5 4.5 15.5"/><path d="M4.5 10.5v5h5"/>`,
  labelSouthEast: `${dot(5, 5, 1.5)}<path d="M6.5 6.5 15.5 15.5"/><path d="M15.5 10.5v5h-5"/>`,
  leader: '<path d="M4 16 10 9.5h2.5"/><path d="M4 16 7 14.2 5.6 12.8z" fill="currentColor" stroke="none"/><path d="M14 8.2h3.2M14 10.8h2.4" stroke-width="1.2"/>',
  // Its arrowheads (Ok's menu) are ARROW_ICONS, below.
  // Bul ve değiştir (docs/adr/0145 §6): the looking glass, and an arrow to what the words become.
  findReplace: '<circle cx="8" cy="8" r="4.3"/><path d="m11.1 11.1 2.4 2.4"/><path d="M10.5 17h6.5m-2-2 2 2-2 2"/>',
  // Okunur yap (docs/adr/0145 §6): a T inside the turning arrow.
  readable: '<path d="M16 10.5A6 6 0 1 1 13.6 5.7"/><path d="M14 2.6v3.5h-3.5"/><path d="M7.5 8.3h5M10 8.3v5"/>',
  dimension: '<path d="M3.5 5v10M16.5 5v10M3.5 12h13"/><path d="m5.8 10.5-2.3 1.5 2.3 1.5M14.2 10.5l2.3 1.5-2.3 1.5"/><path d="M8 8.5h4"/>',
  hatch: '<rect x="3.5" y="3.5" width="13" height="13"/><path d="m3.5 9.5 6-6M3.5 15.5l12-12M9.5 16.5l7-7"/>',
  // docs/adr/0186, the owner's choices (6 October): Çoklu tara two selected hatched areas, a hatch's tie a chain link,
  // a gradient's shapes bands of the ink's share.
  hatchSelected: '<path d="M2.5 3.5h7v6h-7z" stroke-width="1.2"/><path d="M2.5 7.3L4.7 9.5M2.5 4.9L7.1 9.5M3.5 3.5L9.5 9.5M5.9 3.5L9.5 7.1M8.3 3.5L9.5 4.7" stroke-width="1"/><path d="M11 9.5l6.5 1v7H10z" stroke-width="1.2"/><path d="M10.3 15.1L12.7 17.5M10.57 12.97L15.1 17.5M10.83 10.83L17.5 17.5M12.06 9.66L17.5 15.1M14.9 10.1L17.5 12.7" stroke-width="1"/><rect x="1" y="2" width="3" height="3" fill="currentColor" stroke="none"/><rect x="16" y="16" width="3" height="3" fill="currentColor" stroke="none"/>',
  hatchAssoc: '<rect x="3.5" y="3.5" width="13" height="13" rx="1" stroke-width="1.2"/><path d="M3.5 15.5L4.5 16.5M3.5 12.5L7.5 16.5M3.5 9.5L9.2 15.2M3.5 6.5L9.2 12.2M3.5 3.5L11.2 11.2M6.5 3.5L14.2 11.2M9.5 3.5L16.5 10.5M12.5 3.5L16.5 7.5M15.5 3.5L16.5 4.5" stroke-width="1"/><path d="M12.6 13.2h-1.2a1.8 1.8 0 0 0 0 3.6h1.2M14.8 13.2H16a1.8 1.8 0 0 1 0 3.6h-1.2M12.4 15h2.6" stroke-width="1.2"/>',
  hatchGradientLinear: '<rect x="3.5" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".06" stroke="none"/><rect x="6.1" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".16" stroke="none"/><rect x="8.7" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".28" stroke="none"/><rect x="11.3" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".42" stroke="none"/><rect x="13.9" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".58" stroke="none"/><rect x="3.5" y="3.5" width="13" height="13" rx="1" stroke-width="1.2"/>',
  hatchGradientCylinder: '<rect x="3.5" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".08" stroke="none"/><rect x="6.1" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".3" stroke="none"/><rect x="8.7" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".58" stroke="none"/><rect x="11.3" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".3" stroke="none"/><rect x="13.9" y="3.5" width="2.62" height="13" fill="currentColor" fill-opacity=".08" stroke="none"/><rect x="3.5" y="3.5" width="13" height="13" rx="1" stroke-width="1.2"/>',
  hatchGradientSpherical: '<circle cx="10" cy="10" r="6.5" fill="currentColor" fill-opacity=".1" stroke="none"/><circle cx="10" cy="10" r="4.8" fill="currentColor" fill-opacity=".22" stroke="none"/><circle cx="10" cy="10" r="3.1" fill="currentColor" fill-opacity=".38" stroke="none"/><circle cx="10" cy="10" r="1.5" fill="currentColor" fill-opacity=".58" stroke="none"/><rect x="3.5" y="3.5" width="13" height="13" rx="1" stroke-width="1.2"/>',
  // Blocks (docs/adr/0144): a symbol placed at its grip; objects gathered round a base grip; a shelf of symbols;
  // a symbol with its attributes' texts beside it.
  blockInsert: `<rect x="8.5" y="3.5" width="8" height="8" rx="1.2"/><circle cx="12.5" cy="7.5" r="2"/><path d="M3.5 16.5 8.5 11.5"/>${grip(3.5, 16.5)}`,
  blockDefine: `<rect x="3.5" y="3.5" width="13" height="13" rx="1" stroke-dasharray="2.2 1.6"/><circle cx="8" cy="8" r="2"/><path d="m10.8 13.2 2.2-4 2.2 4z"/>${grip(3.5, 16.5)}`,
  blocks: '<rect x="3" y="3" width="6" height="6" rx="1"/><rect x="11" y="3" width="6" height="6" rx="1"/><rect x="3" y="11" width="6" height="6" rx="1"/><circle cx="14" cy="14" r="3"/>',
  blockAttributes: '<rect x="3" y="4.5" width="7" height="7" rx="1.2"/><circle cx="6.5" cy="8" r="1.8"/><path d="M12.5 5.5h5M12.5 8.5h5M12.5 11.5h3.5M3 15.5h14.5"/>',
  // Taban noktasını değiştir: the block's old base (hollow) and where it goes (the grip).
  blockBase: `<rect x="2.5" y="2.5" width="10" height="10" rx="1.2"/><circle cx="7.5" cy="7.5" r="2.2"/><rect x="1.3" y="11.3" width="2.4" height="2.4" stroke-width="1.1"/><path d="M4.2 15.2c2.6 1.8 6.4 2 9.4.7" stroke-width="1.1" stroke-dasharray="1.6 1.4"/>${grip(16, 15.5)}`,
  move: '<path d="M10 2.5v15M2.5 10h15"/><path d="M7.8 4.7 10 2.5l2.2 2.2M7.8 15.3l2.2 2.2 2.2-2.2M4.7 7.8 2.5 10l2.2 2.2M15.3 7.8l2.2 2.2-2.2 2.2"/>',
  copy: '<rect x="3.5" y="7.5" width="9" height="9" rx="1"/><path d="M7.5 7.5v-4h9v9h-4"/>',
  rotate: '<path d="M16 10.5A6 6 0 1 1 13.6 5.7"/><path d="M14 2.6v3.5h-3.5"/>',
  scale: '<rect x="3.5" y="10.5" width="6" height="6"/><path d="M3.5 7.5v-4h13v13h-4"/><path d="m9.5 10.5 6-6M12 4.5h3.5V8"/>',
  mirror: '<path d="M10 2.5v15" stroke-dasharray="2 1.8"/><path d="M7.8 5 3 15h4.8zM12.2 5 17 15h-4.8z"/>',
  offset: '<path d="M3.5 13.5C5 8.5 8.5 5 14.5 4"/><path d="M6.2 16.5c1.4-4.4 4.4-7.3 10-8.2"/>',
  trim: '<path d="M12.5 3v14"/><path d="M3 10h9.5"/><path d="M12.5 10H17" stroke-dasharray="1.6 1.8"/><path d="m14.2 5.8 2.6-2.6M16.8 5.8l-2.6-2.6"/>',
  extend: '<path d="M16.5 3v14"/><path d="M3 10h7"/><path d="M10 10h6.5" stroke-dasharray="1.6 1.8"/><path d="m12.6 7.6 2.4 2.4-2.4 2.4"/>',
  // Buda and Uzat with Çit: the fence, a zigzag, across the lines it cuts or carries on.
  trimFence:
    '<path d="M10 3v14"/><path d="M2.5 7H10M2.5 13H10"/><path d="M10 7h7.5M10 13h7.5" stroke-dasharray="1.6 1.8"/><path d="m15.8 3-2.3 4.6 2.3 4.6-2.3 4.6" stroke-width="1.8"/>',
  extendFence:
    '<path d="M17 3v14"/><path d="M2.5 7H9M2.5 13H9"/><path d="M9 7h8M9 13h8" stroke-dasharray="1.6 1.8"/><path d="m7.6 3-2.3 4.6 2.3 4.6-2.3 4.6" stroke-width="1.8"/>',
  // Köşe yuvarla and Pah: the new edge heavy, the two lines' grips where they are clicked.
  fillet: `<path d="M3.5 16V11M11 3.5H16"/><path d="M3.5 11A7.5 7.5 0 0 1 11 3.5" stroke-width="2.4"/>${grip(3.5, 16.5)}${grip(16.5, 3.5)}`,
  chamfer: `<path d="M3.5 16V11M11 3.5H16"/><path d="M3.5 11 11 3.5" stroke-width="2.4"/>${grip(3.5, 16.5)}${grip(16.5, 3.5)}`,
  break: `<path d="M3 15 8 12M12 9.6l5-3"/>${grip(8, 12)}${grip(12, 9.6)}<path d="m8.8 6.5 2.4 1.6M8.8 15l2.4 1.6" stroke-dasharray="1.4 1.2"/>`,
  join: `<path d="M3 15.5 7.5 7.5M12.5 7.5l4.5 8"/><path d="M7.5 7.5h5" stroke-dasharray="1.6 1.4"/>${grip(7.5, 7.5)}${grip(12.5, 7.5)}`,
  explode: '<path d="M3.5 7.5v-4h4M16.5 7.5v-4h-4M3.5 12.5v4h4M16.5 12.5v4h-4"/><path d="M10 8V6.5M10 13.5V12M8 10H6.5M13.5 10H12"/>',
  stretch: `<path d="M3.5 5.5h6l6 4.5-6 4.5h-6z"/><rect x="8.5" y="2.5" width="9" height="15" stroke-dasharray="1.8 1.6"/>${grip(15.5, 10)}`,
  vertex: `<path d="m3 15 5-9 9 5"/>${grip(3, 15)}${grip(8, 6)}${grip(17, 11)}<path d="M12.5 12.5v5M10 15h5"/>`,
  // Every corner of a closed path rounded or cut, the new edges heavy.
  filletAll:
    '<path d="M8 3.5h4M16.5 8v4M12 16.5H8M3.5 12V8" stroke-width="1.1"/><path d="M3.5 8A4.5 4.5 0 0 1 8 3.5M12 3.5A4.5 4.5 0 0 1 16.5 8M16.5 12a4.5 4.5 0 0 1-4.5 4.5M8 16.5A4.5 4.5 0 0 1 3.5 12" stroke-width="2.4"/>',
  chamferAll:
    '<path d="M8 3.5h4M16.5 8v4M12 16.5H8M3.5 12V8" stroke-width="1.1"/><path d="M3.5 8 8 3.5M12 3.5 16.5 8M16.5 12 12 16.5M8 16.5 3.5 12" stroke-width="2.4"/>',
  reverse: '<path d="M3.5 7h12"/><path d="m13 4.5 2.5 2.5-2.5 2.5"/><path d="M16.5 13h-12"/><path d="m7 10.5-2.5 2.5L7 15.5"/>',
  simplify: `<path d="m3 12 2.2-2.6 1.6 1.4 2.4-3.3 1.6 1.2 2.2-3 1.6 1.1L17 5.5" stroke-dasharray="1.5 1.3"/><path d="M3 16 17 8.5"/>${grip(3, 16)}${grip(17, 8.5)}`,
  split: `<path d="M3 14.5 7.3 12.2M9.2 11.2l1.6-.8M12.7 9.4 17 7.1"/><path d="m7.4 9.4 1.9 4M10.6 7.6l1.9 4" stroke-width="1.1"/>${grip(3, 14.5)}${grip(17, 7.1)}`,
  // Parçala into equal parts (three, the cuts marked) and by a length (measured off from the start).
  splitEqual: `<path d="M2.5 11h4.2M8 11h4M13.3 11h4.2"/><path d="M7.35 7.5v7M12.65 7.5v7" stroke-width="1.1"/>${grip(2.5, 11)}${grip(17.5, 11)}`,
  splitLength: `<path d="M2.5 13h7.6M11.4 13h6.1"/><path d="M2.5 5.5v4M10.75 5.5v9.5M2.5 7.5h8.25" stroke-width="1.1"/><path d="m4.3 6.3-1.8 1.2 1.8 1.2M8.95 6.3l1.8 1.2-1.8 1.2" stroke-width="1.1"/>${grip(2.5, 13)}${grip(17.5, 13)}`,
  cleanup: '<path d="M14.5 2.8 10.4 9"/><path d="M6.4 9.4h7.2l1.2 7.1H5.2z"/><path d="M8.1 12.2v4.3M10 12.2v4.3M11.9 12.2v4.3"/>',
  // Topolojik temizlik (docs/adr/0148): three line ends brought to one node.
  topology: '<circle cx="10" cy="10.5" r="2.3"/><path d="M10 2.8v5.4M3.2 16.6l5.1-4.6M16.8 16.6l-5.1-4.6"/><circle cx="10" cy="10.5" r=".9" fill="currentColor" stroke="none"/>',
  // Topolojik düzenleme (docs/adr/0160): two parcels' shared edge moved from where it was (dashed), its grip at the
  // moved corner; Noktalar da puts a survey point on that corner.
  topologyEdit: `<path d="M2.5 4.5h15v11h-15z"/><path d="M8.5 4.5v11" stroke-dasharray="1.6 1.5" stroke-width="1.1"/><path d="M12.5 4.5 8.5 15.5"/>${grip(12.5, 4.5)}`,
  topologyPoints:
    '<path d="M10.1 4.5H2.5v11h15v-11h-2.6"/><path d="M8.5 4.5v11" stroke-dasharray="1.6 1.5" stroke-width="1.1"/><path d="M11.7 6.75 8.5 15.5"/><circle cx="12.5" cy="4.5" r="2.2" fill="currentColor" fill-opacity=".25"/><circle cx="12.5" cy="4.5" r=".6" fill="currentColor" stroke="none"/>',
  // The overlap control (docs/adr/0162): two areas overlapping; the second cut back (its old edge dashed); one layer; two layers.
  overlapAllow: '<path d="M2.5 3.5h9v9h-9z"/><path d="M8.5 7.5h9v9h-9z"/><path d="M8.5 7.5h3v5h-3z" fill="currentColor" fill-opacity=".3" stroke="none"/>',
  overlap:
    '<path d="M2.5 3.5h9v9h-9z"/><path d="M11.5 7.5h6v9h-9v-4h3z" fill="currentColor" fill-opacity=".14"/><path d="M8.5 12.5v-5h3" stroke-dasharray="1.4 1.4" stroke-width="1.1"/>',
  overlapLayer:
    '<path d="M2.5 2.5h8v8h-8z"/><path d="M10.5 5h6v8.5h-9v-3h3z" fill="currentColor" fill-opacity=".14"/><path d="M2 15.6 5.5 14 9 15.6 5.5 17.2z" fill="currentColor" fill-opacity=".3" stroke-width="1.1"/>',
  overlapLayers:
    '<path d="M2.5 2.5h8v8h-8z"/><path d="M10.5 5h6v8.5h-9v-3h3z" fill="currentColor" fill-opacity=".14"/><path d="M2 14.8 5.5 13.2 9 14.8 5.5 16.4z" fill="currentColor" fill-opacity=".3" stroke-width="1.1"/><path d="M2 17 5.5 18.6 9 17" stroke-width="1.1"/>',
  matchProperties: '<path d="m11.4 3.2 5.4 5.4-5.1 5.1-5.4-5.4z"/><path d="M6.3 8.3 3.2 16.8l8.5-3.1"/><path d="M4.6 12.9 7 15.4"/>',
  setElevation: `<path d="M3 16.5h14"/><path d="M10 13.5V3.5"/><path d="m7 6.5 3-3 3 3"/><path d="M13.5 13.5h3M13.5 10.5h2" stroke-width="1.1"/>${grip(10, 13.5)}`,
  setElevationIncrement: `<path d="M2.5 17h15"/><path d="M5 17v-3.5M10 17v-6.5M15 17v-9.5" stroke-width="1.1"/>${grip(5, 13.5)}${grip(10, 10.5)}${grip(15, 7.5)}<path d="M5.5 3.5v5M3 6h5"/>`,
  setElevationReset: `<path d="M2.5 16h15"/><path d="M10 4v8.5" stroke-dasharray="1.6 1.8"/><path d="m7.2 10.8 2.8 2.8 2.8-2.8"/>${grip(10, 16)}<path d="m13.5 3.5 3 3M16.5 3.5l-3 3" stroke-width="1.1"/>`,
  sector: `<path d="M5 15.5V4.5a11 11 0 0 1 11 11z" fill="currentColor" fill-opacity=".14"/>${grip(5, 15.5)}${grip(5, 4.5)}${grip(16, 15.5)}`,
  pointsBetween: `<path d="M3 14 17 6"/><circle cx="7.7" cy="11.3" r="1.25"/><circle cx="10" cy="10" r="1.25"/><circle cx="12.3" cy="8.7" r="1.25"/>${grip(3, 14)}${grip(17, 6)}`,
  intersectPoint: `<circle cx="7.5" cy="11" r="4.6"/><circle cx="12.5" cy="11" r="4.6"/><circle cx="10" cy="7.15" r="1.4" fill="currentColor"/>${grip(7.5, 11)}${grip(12.5, 11)}`,
  // Çizim ekleri (docs/adr/0197): the options sheet's recommended ones, taken while the owner was away (7 October): two
  // circles and their outer tangent touching them at its grips; a parallelogram's three given corners as grips, the
  // fourth ringed, its two sides to come dashed; a quarter of three rings round the centre's grip between two rays
  // (full rings with rays read as Halka or a target at 16 px).
  tangentLine: `<circle cx="4.6" cy="13.2" r="2.6" stroke-width="1.1"/><circle cx="14.2" cy="11.6" r="4.2" stroke-width="1.1"/><path d="M1.68 11.45 14.92 6.91"/>${grip(3.76, 10.74)}${grip(12.84, 7.63)}`,
  fourthCorner: `<path d="M2.5 16h9l6-11"/><path d="M17.5 5h-9l-6 11" stroke-dasharray="2.2 1.6" stroke-width="1.2"/>${grip(2.5, 16)}${grip(11.5, 16)}${grip(17.5, 5)}<circle cx="8.5" cy="5" r="2" stroke-width="1.3"/>`,
  rangeRings: `<path d="M4 11.5A4.5 4.5 0 0 1 8.5 16M4 7.5A8.5 8.5 0 0 1 12.5 16M4 3.5A12.5 12.5 0 0 1 16.5 16" stroke-width="1.2"/><path d="M4 16V3.5M4 16h12.5" stroke-width="1.1"/>${grip(4, 16)}`,
  // Plan yolu çizimi (docs/adr/0198), taken while the owner was away (7 October): the road an area, lightly filled between
  // its edges with its axis dashed (the first option read as Paralel çizgi); a crossing's four rounded corners; a median
  // closed at both ends.
  planRoad: `<path d="M3.6 18.92 10.53 12.91 19.31 10.59 17.69 4.41 7.47 7.09 -0.6 14.08" fill="currentColor" fill-opacity=".16" stroke="none"/><path d="M3.6 18.92 10.53 12.91 19.31 10.59"/><path d="M-0.6 14.08 7.47 7.09 17.69 4.41"/><path d="M1.5 16.5 9 10 18.5 7.5" stroke-dasharray="2.2 1.6" stroke-width="1.1"/>`,
  roadJunctions: `<path d="M7 1.5V4.5A2.5 2.5 0 0 1 4.5 7H1.5"/><path d="M13 1.5V4.5A2.5 2.5 0 0 0 15.5 7H18.5"/><path d="M7 18.5V15.5A2.5 2.5 0 0 0 4.5 13H1.5"/><path d="M13 18.5V15.5A2.5 2.5 0 0 1 15.5 13H18.5"/>`,
  medianClose: `<path d="M6 7H14A3 3 0 0 1 14 13H6A3 3 0 0 1 6 7z"/>`,
  // Ara nokta by a distance along (measured) and by a ratio (%).
  pointsBetweenDistance: `<path d="M3 16.5 17 8.5"/><circle cx="10" cy="12.5" r="1.3" fill="currentColor" stroke="none"/><circle cx="14.2" cy="10.1" r="1.3" fill="currentColor" stroke="none"/>${grip(3, 16.5)}${grip(17, 8.5)}<path d="M2.2 13.2 9.2 9.2M2.6 11.6l-.4 1.6 1.6.4M7.6 8.8l1.6.4-.4 1.6" stroke-width="1.1"/>`,
  pointsBetweenRatio: `<path d="M3 16 17 8"/><circle cx="10" cy="12" r="1.25" fill="currentColor" stroke="none"/>${grip(3, 16)}${grip(17, 8)}<circle cx="5" cy="4.5" r="1.3" stroke-width="1.1"/><circle cx="10" cy="7.5" r="1.3" stroke-width="1.1"/><path d="M10.5 3.5 4.5 8.5" stroke-width="1.1"/>`,
  // Kesişim noktası from two directions (their angles marked) and of two lines (two points each).
  intersectDirections: `<path d="M3.5 16.5 11 3.5M16.5 16.5 9 3.5"/><path d="M6.5 16.5a3 3 0 0 0-1.4-2.6M13.5 16.5a3 3 0 0 1 1.4-2.6" stroke-width="1.1"/>${grip(3.5, 16.5)}${grip(16.5, 16.5)}<circle cx="10" cy="5.2" r="1.5" fill="currentColor" stroke="none"/>`,
  intersectLines: `<path d="M2.5 15.5 17.5 5.5M2.5 5.5l15 10"/>${grip(2.5, 15.5)}${grip(6.5, 12.83)}${grip(2.5, 5.5)}${grip(6.5, 8.17)}<circle cx="10" cy="10.5" r="1.5" fill="currentColor" stroke="none"/>`,
  measureAngle: `<path d="M3.5 16h13M3.5 16 13 5"/><path d="M9.5 16a6 6 0 0 0-2.1-4.55"/><path d="M11.4 13.2h2.4" stroke-width="1.1"/>${grip(3.5, 16)}`,
  stationOffset: `<path d="M3 13h14"/><path d="M12 13V5.8" stroke-dasharray="2 1.5"/><path d="M12 10.8h2.2V13"/><circle cx="12" cy="4.6" r="1.4"/><path d="M3 16.2h9M3 15v2.4M12 15v2.4" stroke-width="1.1"/>${grip(3, 13)}${grip(17, 13)}`,
  dimContinue: '<path d="M3 5.5v10M10 5.5v10M17 5.5v10M3 12.5h14"/><path d="m5.1 11.3-2.1 1.2 2.1 1.2M7.9 11.3l2.1 1.2-2.1 1.2M12.1 11.3l-2.1 1.2 2.1 1.2M14.9 11.3l2.1 1.2-2.1 1.2"/>',
  dimBaseline: '<path d="M3.5 4v12.5M10 10v6.5M16.5 5.5v11M3.5 13h6.5M3.5 8h13"/><path d="m5.5 12-2 1 2 1M8 12l2 1-2 1M5.5 7l-2 1 2 1M14.5 7l2 1-2 1"/>',
  // Ölçülendirme's methods, each its own drawing (docs/adr/0147 §7): the measured thing solid, the dimension's lines
  // thin with open arrowheads, its value a short bar. Hizalı: a slanted edge, its dimension line parallel to it.
  dimAligned: '<path d="M5 16.5 16.5 8.5"/><path d="M4.43 15.68 2.03 12.23M15.93 7.68 13.53 4.23M2.54 12.97 14.04 4.97" stroke-width="1.1"/><path d="M4.84 12.77 2.54 12.97 3.53 10.88M11.75 5.17 14.04 4.97 13.06 7.06" stroke-width="1.1"/><path d="M5.67 8.36 8.63 6.3" stroke-width="1.5"/>',
  // Doğrusal: a slanted edge, its dimension line level (ΔY).
  dimLinear: '<path d="M4 16.5 16 10.5"/><path d="M4 15V4.6M16 9V4.6M4 5.6h12" stroke-width="1.1"/><path d="M6 6.75 4 5.6 6 4.45M14 4.45 16 5.6 14 6.75" stroke-width="1.1"/><path d="M8.3 3.3 11.7 3.3" stroke-width="1.5"/>',
  // Açı: two arms from a vertex, the dimension arc between them.
  dimAngular: '<path d="M3.5 16.5H17.5M3.5 16.5 11.33 4.89"/><path d="M9.09 8.21A10 10 0 0 1 13.5 16.5" stroke-width="1.1"/><path d="M14.65 14.5 13.5 16.5 12.35 14.5M10.11 10.28 9.09 8.21 11.39 8.37" stroke-width="1.1"/><rect x="2" y="15" width="3" height="3" fill="currentColor" stroke="none"/>',
  // Yarıçap: a circle, a line from its centre with one arrow on the circle.
  dimRadius: '<circle cx="8" cy="12" r="5.8"/><path d="M8 12 15.43 5.31h2.6" stroke-width="1.1"/><path d="M10.05 8.6 12.31 8.12 11.59 10.31" stroke-width="1.1"/><rect x="6.5" y="10.5" width="3" height="3" fill="currentColor" stroke="none"/>',
  // Çap: a circle, a line across it through its centre, an arrow at each end.
  dimDiameter: '<circle cx="10" cy="10" r="6.6"/><path d="M4.59 13.79 15.41 6.21" stroke-width="1.1"/><path d="M6.89 13.58 4.59 13.79 5.57 11.7M13.11 6.42 15.41 6.21 14.43 8.3" stroke-width="1.1"/><circle cx="10" cy="10" r=".9" fill="currentColor" stroke="none"/>',
  // Koordinat: a point, its Y line up and its X line left, each with its value.
  dimOrdinate: '<path d="M12.5 11.6V3.5M10.6 13.5H2.5" stroke-width="1.1"/><path d="M10.2 4.2 10.2 8.6" stroke-width="1.5"/><path d="M3.4 11.2 7.8 11.2" stroke-width="1.5"/><rect x="11" y="12" width="3" height="3" fill="currentColor" stroke="none"/>',
  // Yay uzunluğu: an arc, the dimension arc round it, the arc mark over the value.
  dimArcLength: '<path d="M4.09 13.38A7.5 7.5 0 0 1 15.91 13.38"/><path d="M3.38 12.83 1.02 10.98M16.62 12.83 18.98 10.98M1.65 11.47A10.6 10.6 0 0 1 18.35 11.47" stroke-width="1.1"/><path d="M3.78 10.61 1.65 11.47 1.97 9.19M18.03 9.19 18.35 11.47 16.22 10.61" stroke-width="1.1"/><path d="M8.4 4.4a1.6 1.6 0 0 1 3.2 0" stroke-width="1.2"/>',
  // Kırıklı yarıçap: a large arc, the radius from a centre shown near it, jogged.
  dimJogged: '<path d="M10.5 2.8A14.5 14.5 0 0 1 17.3 15.4"/><path d="M3.5 16 7.6 13.6 7 11.1 11.3 8.9 15.1 6.8" stroke-width="1.1"/><path d="M12.79 6.76 15.1 6.8 13.91 8.77" stroke-width="1.1"/><rect x="2" y="14.5" width="3" height="3" fill="currentColor" stroke="none"/>',
  // Semt: north up from a station, the edge, the bearing turning clockwise from north.
  dimAzimuth: '<path d="M6.5 16.5 17.52 9.61"/><path d="M6.5 15V2.8" stroke-dasharray="1.6 1.4" stroke-width="1.1"/><path d="m4.6 5 1.9-2.6L8.4 5" stroke-width="1.2"/><path d="M6.5 9.1A7.4 7.4 0 0 1 12.78 12.58" stroke-width="1.1"/><path d="M12.69 10.27 12.78 12.58 10.74 11.49" stroke-width="1.1"/><rect x="5" y="15" width="3" height="3" fill="currentColor" stroke="none"/>',
  // Eğim: the ground, an arrow down the slope over it, its percent.
  dimSlope: '<path d="M2.5 9.5 17.5 16"/><path d="M5.6 6 14.6 9.9" stroke-width="1.1"/><path d="M13.22 8.05 14.6 9.9 12.31 10.16" stroke-width="1.1"/><path d="m13.2 6.9 4-5" stroke-width="1.1"/><circle cx="13.6" cy="2.6" r=".95" stroke-width="1.1"/><circle cx="16.9" cy="6.1" r=".95" stroke-width="1.1"/>',
  // Hızlı ölçü: an area dimensioned along two sides at once.
  dimQuick: '<path d="M3.5 9h9.5v7.5H3.5z" fill="currentColor" fill-opacity=".14"/><path d="M3.5 5.5H13M16.5 9v7.5" stroke-width="1.1"/><path d="M5.3 6.6 3.5 5.5 5.3 4.4M11.2 4.4 13 5.5 11.2 6.6M15.4 10.8 16.5 9 17.6 10.8M17.6 14.7 16.5 16.5 15.4 14.7" stroke-width="1.1"/>',
  arrayPath: '<path d="M3 16c2.7-6.4 7.4-10.3 14-11" stroke-dasharray="2 1.6"/><rect x="2" y="12.5" width="3.2" height="3.2"/><rect x="7" y="7.6" width="3.2" height="3.2"/><rect x="13" y="4" width="3.2" height="3.2"/>',
  divide: `<path d="M3 13.5 17 6.5"/><circle cx="7.7" cy="11.2" r="1.3"/><circle cx="12.3" cy="8.8" r="1.3"/>${grip(3, 13.5)}${grip(17, 6.5)}`,
  array: '<rect x="3.5" y="3.5" width="5" height="5"/><rect x="11.5" y="3.5" width="5" height="5"/><rect x="3.5" y="11.5" width="5" height="5"/><rect x="11.5" y="11.5" width="5" height="5"/>',
  erase: '<path d="M8.5 16.5h8"/><path d="m11.4 3.9 4.7 4.7a1 1 0 0 1 0 1.4L9.7 16.5H6.4l-2.7-2.7a1 1 0 0 1 0-1.4l6.3-8.5a1 1 0 0 1 1.4 0z"/><path d="m7.3 8.2 4.5 4.5"/>',
  parcel: `<path d="m3.5 7 6.5-3.5 6.5 3.4-1.4 9.6H5.1z"/><path d="M8.3 12.2h3.4M10 10.5v3.4"/>${grip(3.5, 7)}${grip(10, 3.5)}${grip(16.5, 6.9)}${grip(15.1, 16.5)}${grip(5.1, 16.5)}`,
  subdivide: '<path d="M3.5 5.5h13v9h-13z"/><path d="m11.2 5.5-2.2 9" stroke-dasharray="2.2 1.6"/>',
  stakeout: '<circle cx="10" cy="6.5" r="3.3"/><path d="M10 3.2v6.6M6.7 6.5h6.6M10 9.8 5.8 17M10 9.8l4.2 7.2M10 9.8V17"/>',
  spot: '<path d="M10 4 4.8 13h10.4z"/><path d="M3 16.5h14"/><circle cx="10" cy="10" r=".9" fill="currentColor"/>',
  measure: '<path d="M2.8 13.6 13.6 2.8l3.6 3.6L6.4 17.2z"/><path d="m5.9 10.5 1.6 1.6M8.4 8l2.2 2.2M10.9 5.5l1.6 1.6"/>',
  area: '<path d="M3.5 16.5v-13h13v13z" stroke-dasharray="2 1.7"/><path d="M7 13V7h6v6z" fill="currentColor" fill-opacity=".35"/>',
  parallel: '<path d="M2 14 9 7h9" stroke-dasharray="2.4 1.6"/><path d="M2 10.46 7.96 4.5H18M2 17.54 10.04 9.5H18"/>',
  perpIn: `<path d="M3 16h14M10 4v12M10 13.3h2.7V16"/><path d="m8 8.5 2 2 2-2"/>${grip(10, 4)}`,
  surveyTraverse: `<path d="m3 15 4.5-7 5 3.5L17 4"/><path d="M9.3 9.6a2.4 2.4 0 0 1-2.1 1"/>${grip(3, 15)}${grip(17, 4)}<circle cx="7.5" cy="8" r="1.1"/><circle cx="12.5" cy="11.5" r="1.1"/>`,
  // A field book: its binding, two lines of readings and an angle.
  // Cihaza gönder: an arrow into a total station on its tripod.
  fieldSend: `<rect x="9.5" y="3.5" width="5.5" height="6" rx="1"/><circle cx="12.25" cy="6.5" r="1.1"/><path d="M12.25 9.5 9 17.5M12.25 9.5l3.25 8M12.25 9.5v8"/><path d="M1.8 6.5h5.4M5.2 4.5l2 2-2 2"/>`,
  fieldBook: `<rect x="5" y="2.5" width="11" height="15" rx="1.5"/><path d="M5 6H3.5M5 10H3.5M5 14H3.5"/><path d="M8 6.5h5M8 9.5h5"/><path d="M8 14.5l4-2.6M8 14.5h4.5"/>`,
  surveyPolar: `<path d="M5 15 15 5M5 15l11 1"/><path d="M5 15 4 3.5" stroke-dasharray="2 1.5"/><path d="M4.6 10.4a4.6 4.6 0 0 1 3.7 1.3"/>${grip(5, 15)}<circle cx="15" cy="5" r="1.3"/><circle cx="16" cy="16" r="1.3"/>`,
  surveyStakeout: `<path d="M4 15 13 8.2" stroke-dasharray="2.4 1.6"/><path d="M15 4.5v10.5M13 15h4"/><path d="m10.3 7.9 2.7.3-1 2.5"/>${grip(4, 15)}`,
  surveyForward: `<path d="M4 15 10 5l6 10"/><path d="M6.6 15a2.6 2.6 0 0 0-.9-2.1M13.4 15a2.6 2.6 0 0 1 .9-2.1"/>${grip(4, 15)}${grip(16, 15)}<circle cx="10" cy="5" r="1.6"/>`,
  surveyResection: `<path d="M10 15 3.5 5.5M10 15V3.5M10 15l6.5-9.5"/><path d="M7.9 12a2.6 2.6 0 0 1 2.1-.6"/>${grip(3.5, 5.5)}${grip(10, 3.5)}${grip(16.5, 5.5)}<circle cx="10" cy="15" r="1.6"/>`,
  // Yatay ağ dengelemesi (docs/adr/0203): a braced quadrilateral, two fixed points, a new one's error ellipse.
  surveyNetwork: `<path d="M3.5 16 16.5 16M3.5 16 6 5.5M16.5 16 14 6.5M6 5.5 14 6.5M3.5 16 14 6.5M16.5 16 6 5.5" stroke-width="1.1"/>${grip(3.5, 16)}${grip(16.5, 16)}${dot(6, 5.5, 1.2)}<ellipse cx="14" cy="6.5" rx="3.4" ry="1.7" transform="rotate(-28 14 6.5)" stroke-width="1.3"/>${dot(14, 6.5, 0.9)}`,
  // Kot ağı dengelemesi: two staffs, the level between them and its horizontal sight.
  surveyLevel: '<path d="M3 3.5v13M17 6v10.5" stroke-width="1.6"/><path d="M3 6.5h1.5M3 9.5h1.5M3 12.5h1.5M15.5 9h1.5M15.5 12h1.5" stroke-width="1"/><path d="M5 8.2h10" stroke-dasharray="1.6 1.3" stroke-width="1"/><rect x="8" y="6.8" width="4" height="2.8" rx=".6" stroke-width="1.2"/><path d="M10 9.6 7.6 16.5M10 9.6v6.9M10 9.6l2.4 6.9" stroke-width="1"/>',
  perpOut: `<path d="M3 16h14M8 16V4M8 13.3h2.7V16"/><path d="m6 6 2-2 2 2"/>${grip(8, 16)}`,
  arrayPolar: '<circle cx="10" cy="10" r="6.5" stroke-dasharray="2 2"/><rect x="8.3" y="1.8" width="3.4" height="3.4"/><rect x="14.8" y="8.3" width="3.4" height="3.4"/><rect x="8.3" y="14.8" width="3.4" height="3.4"/><rect x="1.8" y="8.3" width="3.4" height="3.4"/><circle cx="10" cy="10" r=".9" fill="currentColor" stroke="none"/>',
  align: `<path d="M4 9.5 8.5 4.5l4 3.6" stroke-dasharray="2 1.5"/><path d="M4 16h9v-5"/>${grip(4, 16)}${grip(13, 16)}`,
  // Hizala ve dağıt (docs/adr/0194): the boxes as framed bars and the reference's line (the option sheet's A).
  arrangeLeft: '<path d="M3 2.5v15"/><rect x="5.5" y="4.5" width="11" height="4" rx="0.8"/><rect x="5.5" y="11.5" width="7" height="4" rx="0.8"/>',
  arrangeCenter: '<path d="M10 2v3m0 3.5v3.5m0 3.5V18"/><rect x="4.5" y="5" width="11" height="3.5" rx="0.8"/><rect x="6.5" y="12" width="7" height="3.5" rx="0.8"/>',
  arrangeRight: '<path d="M17 2.5v15"/><rect x="3.5" y="4.5" width="11" height="4" rx="0.8"/><rect x="7.5" y="11.5" width="7" height="4" rx="0.8"/>',
  arrangeTop: '<path d="M2.5 3h15"/><rect x="4.5" y="5.5" width="4" height="11" rx="0.8"/><rect x="11.5" y="5.5" width="4" height="7" rx="0.8"/>',
  arrangeMiddle: '<path d="M2 10h3m3.5 0H12m3.5 0H18"/><rect x="5" y="4.5" width="3.5" height="11" rx="0.8"/><rect x="12" y="6.5" width="3.5" height="7" rx="0.8"/>',
  arrangeBottom: '<path d="M2.5 17h15"/><rect x="4.5" y="3.5" width="4" height="11" rx="0.8"/><rect x="11.5" y="7.5" width="4" height="7" rx="0.8"/>',
  arrangeHorizontal: '<path d="M2 3v14M18 3v14"/><rect x="4" y="6" width="2.6" height="8" rx="0.8"/><rect x="8.7" y="4" width="2.6" height="12" rx="0.8"/><rect x="13.4" y="7" width="2.6" height="6" rx="0.8"/>',
  arrangeVertical: '<path d="M3 2h14M3 18h14"/><rect x="6" y="4" width="8" height="2.6" rx="0.8"/><rect x="4" y="8.7" width="12" height="2.6" rx="0.8"/><rect x="7" y="13.4" width="6" height="2.6" rx="0.8"/>',
  // Eğri boyunca yazı (docs/adr/0196 §4), the recommended options while the owner was away (7 October): a T whose bar
  // bends, a T going down onto an arc, a T turned along a slanted edge, a curved bar made straight.
  textAlong: '<path d="M2.8 10.2C4.6 3 15.4 3 17.2 10.2"/><path d="M10 5.4v11.1M7.6 16.5h4.8"/>',
  textFit: '<path d="M6.5 3h7M10 3v4.4"/><path d="m8.4 8.6 1.6 1.6 1.6-1.6" stroke-width="1.2"/><path d="M3 17.5C5.5 12 14.5 12 17 17.5"/><path d="M8 13.4h4" stroke-width="1.2"/>',
  textTurn: '<path d="M3 16.5 17 5.5" stroke-width="1.1"/><g transform="rotate(-38 10 13)"><path d="M7 13h6M10 13v4.5"/></g><path d="M5.5 3.5h5M8 3.5v4" stroke-width="1.1" opacity=".55"/>',
  textStraighten: '<path d="M3 6C6 2.8 14 2.8 17 6" stroke-width="1.2" stroke-dasharray="2 1.6"/><path d="M4.5 9.5h11M10 9.5v8"/><path d="m8.4 5.4 1.6 1.6 1.6-1.6" stroke-width="1.2"/>',
  lengthen: `<path d="M3 13h9"/><path d="M12 13h5.5" stroke-dasharray="2 1.5"/><path d="m15 10.5 2.5 2.5-2.5 2.5"/>${grip(12, 13)}`,
  // Sürdür (docs/adr/0173 §4): a polyline drawn, its end's grip, the new vertices going on from it dashed.
  continue: `<path d="M2.5 16.5 6 10.5l4 3"/><path d="m10 13.5 3.5-6.5 4 1.5" stroke-dasharray="1.8 1.4"/>${grip(10, 13.5)}${grip(13.5, 7)}${grip(17.5, 8.5)}`,
  // Biçim değiştir (docs/adr/0173 §2): an area whose side (dashed) gives way to the line drawn over it.
  reshape: '<path d="M11.5 3.5h-8v13h8"/><path d="M11.5 3.5v13" stroke-dasharray="1.6 1.3"/><path d="M11.5 3.5 17 6.5l-2.5 3.5 2.5 3.5-5.5 3"/>',
  donut: '<path d="M10 3.5a6.5 6.5 0 1 1 0 13 6.5 6.5 0 1 1 0-13zm0 3.5a3 3 0 1 0 0 6 3 3 0 1 0 0-6z" fill="currentColor" fill-opacity=".35" fill-rule="evenodd"/>',
  revcloud: '<path d="M5 8.2a2.2 2.2 0 0 1 3.5-2 2.4 2.4 0 0 1 4-.2 2.2 2.2 0 0 1 3.4 1.9 2.2 2.2 0 0 1 .3 4.2 2.3 2.3 0 0 1-3 3 2.4 2.4 0 0 1-4 .3 2.3 2.3 0 0 1-3.6-1.9A2.2 2.2 0 0 1 5 8.2z"/>',
  // Alan işlemleri
  boundary: '<path d="M3 15h14M5 17 11 3M9 3l6 14"/><path d="M5.86 15h8.28L10 5.33z" fill="currentColor" fill-opacity=".3" stroke="none"/><circle cx="10" cy="11.8" r="1" fill="currentColor" stroke="none"/>',
  // Toplu alan (docs/adr/0151): a parcel network's cells made areas, each with its number.
  polygonize: '<path d="M2.5 4.5 17.5 3v14L2.5 16zM9 3.8 10.5 16.5M2.5 10.5l15-1"/><path d="M2.5 4.5 9 3.8l.66 6.1-7.16.6zM10.5 16.5l7 .5v-7.5l-7.6.5z" fill="currentColor" fill-opacity=".3" stroke="none"/><circle cx="5.8" cy="7.3" r=".95" fill="currentColor" stroke="none"/><circle cx="13.8" cy="13.2" r=".95" fill="currentColor" stroke="none"/><circle cx="13.6" cy="6.4" r=".95" fill="currentColor" stroke="none"/>',
  vertexPoints: '<path d="M3.99 13.23 5.01 6.97M7.79 4.38 13.81 3.82M16.28 6 16.52 12.8M14.2 15.27 6 15.53" stroke-width="1.1"/><circle cx="3.6" cy="15.6" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="3.6" cy="15.6" r=".55" fill="currentColor" stroke="none"/><circle cx="5.4" cy="4.6" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="5.4" cy="4.6" r=".55" fill="currentColor" stroke="none"/><circle cx="16.2" cy="3.6" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="16.2" cy="3.6" r=".55" fill="currentColor" stroke="none"/><circle cx="16.6" cy="15.2" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="16.6" cy="15.2" r=".55" fill="currentColor" stroke="none"/>',
  pointEditor: '<path d="M2.5 4.5h11M2.5 8.5h8M2.5 12.5h6M2.5 16.5h6" stroke-width="1.2"/><circle cx="14.5" cy="13.5" r="3.2"/><path d="M14.5 8.8v9.4M9.8 13.5h9.4" stroke-width="1.1"/>',
  // Nokta editörü's batch operations (docs/adr/0153 §5): Yeniden adlandır, Sıralı numara ver, Katmana taşı, Çift noktaları ayıkla.
  pointRename: '<circle cx="4.6" cy="15" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="4.6" cy="15" r=".55" fill="currentColor" stroke="none"/><path d="M8.6 4.4h7.2a1.3 1.3 0 0 1 1.3 1.3v3.6a1.3 1.3 0 0 1-1.3 1.3H8.6L6 7.5z"/><circle cx="8.6" cy="7.5" r=".7" fill="currentColor" stroke="none"/><path d="M5.6 13.1 7 10.6" stroke-width="1.1"/>',
  pointNumber: '<circle cx="3.6" cy="15" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="3.6" cy="15" r=".55" fill="currentColor" stroke="none"/><circle cx="10" cy="15" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="10" cy="15" r=".55" fill="currentColor" stroke="none"/><circle cx="16.4" cy="15" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="16.4" cy="15" r=".55" fill="currentColor" stroke="none"/><path d="M3.6 9.4V4.6M8.6 4.6h2.6v2.3H8.8v2.5h2.6M14.6 4.6h2.6v2.4h-2M17.2 7v2.4h-2.6" stroke-width="1.1"/>',
  pointLayer: '<circle cx="10" cy="3.6" r="2" fill="currentColor" fill-opacity=".25"/><circle cx="10" cy="3.6" r=".55" fill="currentColor" stroke="none"/><path d="M10 6.2v3.6M8.3 8.2 10 9.9l1.7-1.7" stroke-width="1.2"/><path d="m10 11 7 3.3-7 3.3-7-3.3z" fill="currentColor" fill-opacity=".14"/>',
  // Vektör oturtma (docs/adr/0156): a dashed sheet carried by its control points onto the solid one.
  // Kenar eşleme (docs/adr/0159): the sheets' dashed edge, two pairs of lines a little apart, the grips where they meet.
  edgematch: `<path d="M10 2.5v15" stroke-dasharray="2 1.6"/><path d="M2.5 6.8 8.6 7.3M11.4 7.9l6.1.5"/><path d="m2.5 14.2 6.1-1.1M11.4 12.4l6.1-1.1"/>${grip(10, 7.6)}${grip(10, 12.8)}`,
  vectorFit:
    '<path d="m2.6 9.4 6.6-1.2 1.4 7.4-6.8 1.6z" stroke-dasharray="1.6 1.2"/><path d="M11 2.6h6.6v6.6H11z" fill="currentColor" fill-opacity=".18"/><path d="M5.6 7.6 10.2 3.2M10.2 3.2H7.8M10.2 3.2v2.4" stroke-width="1.1"/><circle cx="2.6" cy="9.4" r="1.15" fill="currentColor" stroke="none"/><circle cx="11" cy="2.6" r="1.15" fill="currentColor" stroke="none"/><circle cx="17.6" cy="9.2" r="1.15" fill="currentColor" stroke="none"/>',
  pointDedupe: '<circle cx="5" cy="10" r="2.6"/><circle cx="8.4" cy="10" r="2.6" stroke-dasharray="1 1.1"/><path d="M12.1 10h2.6M13.6 8.6 15 10l-1.4 1.4" stroke-width="1.2"/><circle cx="17.6" cy="10" r="1.9" fill="currentColor" fill-opacity=".25"/><circle cx="17.6" cy="10" r=".55" fill="currentColor" stroke="none"/>',
  toArea: '<path d="m3.8 8 6-4.5 6.5 3.5-1.8 9H5.6z" fill="currentColor" fill-opacity=".3" stroke-dasharray="2.2 1.5"/>',
  areaUnion: '<path d="M3 3h9v5h5v9H8v-5H3z" fill="currentColor" fill-opacity=".22"/>',
  areaIntersect: '<path d="M3 3h9v9H3zM8 8h9v9H8z" stroke-dasharray="2 1.6"/><path d="M8 8h4v4H8z" fill="currentColor" fill-opacity=".55"/>',
  areaSubtract: '<path d="M3 3h14v7h-6v7H3z" fill="currentColor" fill-opacity=".22"/><path d="M11 10h6v7h-6" stroke-dasharray="2 1.6"/>',
  areaSplit: '<path d="M4 4h12v12H4z" fill="currentColor" fill-opacity=".16"/><path d="M2 14.5 18 5.5"/>',
  partsJoin: '<path d="M3 3h6v6H3z" fill="currentColor" fill-opacity=".22"/><path d="M11 11h6v6h-6z" fill="currentColor" fill-opacity=".22"/><path d="M9 9l2 2"/>',
  partsSplit: '<path d="M2 2h13L2 15z" fill="currentColor" fill-opacity=".22"/><path d="M18 5v13H5z" fill="currentColor" fill-opacity=".22"/>',
  toPolyline: `<path d="M5.6 16 3.8 8l6-4.5 6.5 3.5-1.8 9H8"/>${grip(3.8, 8)}${grip(9.8, 3.5)}${grip(16.3, 7)}${grip(14.5, 16)}${grip(5.6, 16)}`,
  // Delikler (docs/adr/0173 §5): the area with its hole, a plus by it (the ring dashed while drawn), a cross by it (the hole gone, its ghost dashed), or the hole filled anew.
  holeAdd: '<path d="M2.5 2.5h11v11h-11zM5.5 5.5h5v5h-5z" fill="currentColor" fill-opacity=".22" fill-rule="evenodd" stroke="none"/><path d="M2.5 2.5h11v11h-11z"/><path d="M5.5 5.5h5v5h-5z" stroke-dasharray="1.5 1.1"/><path d="M16 12.5v6M13 15.5h6"/>',
  holeRemove: '<path d="M2.5 2.5h11v11h-11z" fill="currentColor" fill-opacity=".22"/><path d="M5.5 5.5h5v5h-5z" stroke-dasharray="1.5 1.1"/><path d="m14.5 14.5 4.5 4.5m0-4.5-4.5 4.5"/>',
  holeFill: '<path d="M2.5 2.5h15v15h-15zM6.5 6.5h7v7h-7z" fill="currentColor" fill-opacity=".22" fill-rule="evenodd"/><path d="M6.5 6.5h7v7h-7z" fill="currentColor" fill-opacity=".75"/>',

  // Object snaps: the marker (solid) as drawn on the canvas, on its context geometry (dashed).
  snapEndpoint: '<path d="M3.5 16.5 11 9" stroke-dasharray="2 1.6"/><rect x="10.5" y="3.5" width="6" height="6"/>',
  snapMidpoint: '<path d="M3 16.5 6.6 13M13.4 7 17 3.5" stroke-dasharray="2 1.6"/><path d="m10 5.8 4 7H6z"/>',
  snapCenter: '<circle cx="10" cy="10" r="7" stroke-dasharray="2 1.6"/><circle cx="10" cy="10" r="2.8"/>',
  snapNode: '<circle cx="10" cy="10" r="5"/><path d="m7.2 7.2 5.6 5.6m0-5.6-5.6 5.6"/>',
  snapQuadrant: '<circle cx="10" cy="11.5" r="6" stroke-dasharray="2 1.6"/><path d="m10 2.2 3.3 3.3L10 8.8 6.7 5.5z"/>',
  snapIntersection: '<path d="m3 4.5 14 11M3 15.5l14-11" stroke-dasharray="2 1.6"/><path d="m7 7 6 6m0-6-6 6"/>',
  snapPerpendicular: '<path d="M3 16.5h14" stroke-dasharray="2 1.6"/><path d="M6.5 16.5V5.5M6.5 11.5h5v5"/>',
  snapTangent: '<circle cx="10" cy="12" r="5"/><path d="M3 6.5h14"/>',
  snapNearest: '<path d="M3 16.5 17 3.5" stroke-dasharray="2 1.6"/><path d="M6 6h8l-8 8h8z"/>',
  // The snap additions (docs/adr/0163 §1).
  snapCentroid: '<path d="M3 6 13.5 3 17 12l-5.5 5L3 14z" stroke-dasharray="2 1.6"/><path d="m10 6.6 3.4 3.4-3.4 3.4-3.4-3.4z"/><circle cx="10" cy="10" r="1" fill="currentColor" stroke="none"/>',
  snapExtension: '<path d="M2.5 17 7.5 12"/><path d="m7.5 12 10-10" stroke-dasharray="2 1.6"/><path d="M13.6 3.9v5.4M10.9 6.6h5.4"/>',
  snapParallel: '<path d="M2.5 13.5 10 3" stroke-dasharray="2 1.6"/><path d="m8 17.5 6-8.4M11.8 17.5l6-8.4"/>',
  snapGrid: '<g fill="currentColor" stroke="none"><circle cx="3.5" cy="3.5" r=".95"/><circle cx="10" cy="3.5" r=".95"/><circle cx="16.5" cy="3.5" r=".95"/><circle cx="3.5" cy="10" r=".95"/><circle cx="16.5" cy="10" r=".95"/><circle cx="3.5" cy="16.5" r=".95"/><circle cx="10" cy="16.5" r=".95"/><circle cx="16.5" cy="16.5" r=".95"/></g><path d="M10 6.6v6.8M6.6 10h6.8"/>',
  calc: '<rect x="4" y="2.5" width="12" height="15" rx="1.5"/><path d="M6.5 5.5h7v3h-7z"/><path d="M7 11.5h.01M10 11.5h.01M13 11.5h.01M7 14.5h.01M10 14.5h.01M13 14.5h.01" stroke-width="2" stroke-linecap="round"/>',
  // Point calculator: clicked points are grips, the computed point is a ring.
  calcSide: `<path d="M3 14h14"/><path d="M9 14V7.6" stroke-dasharray="2 1.5"/><path d="M9 11.8h2.2V14"/><circle cx="9" cy="6" r="1.6"/>${grip(3, 14)}${grip(17, 14)}`,
  calcDistances: `<path d="M5 15 10 7.5 15 15" stroke-dasharray="2 1.5"/><path d="M8.33 6.64A9 9 0 0 1 11.44 8.71M8.56 8.71A9 9 0 0 1 11.67 6.64"/><circle cx="10" cy="7.5" r="1.3"/>${grip(5, 15)}${grip(15, 15)}`,
  calcLines: `<path d="M3 15 7 11M17 15l-4-4"/><path d="M7 11 12.5 5.5M13 11 7.5 5.5" stroke-dasharray="2 1.5"/><circle cx="10" cy="8" r="1.5"/>${grip(3, 15)}${grip(7, 11)}${grip(17, 15)}${grip(13, 11)}`,
  calcAlong: `<path d="M3 13h14M3 8.5v2.5M11 8.5v2.5M3 9.7h8"/><circle cx="11" cy="13" r="1.6"/>${grip(3, 13)}${grip(17, 13)}`,
  calcPolar: `<path d="M5 15h12" stroke-dasharray="2 1.5"/><path d="M5 15 11.2 7.1M9 15a4 4 0 0 0-1.54-3.15"/><circle cx="12" cy="6" r="1.5"/>${grip(5, 15)}${grip(17, 15)}`,
  calcMid: `<path d="M3 13h14M6.5 11.3v3.4M13.5 11.3v3.4"/><circle cx="10" cy="13" r="1.6"/>${grip(3, 13)}${grip(17, 13)}`,
  // Nokta hesaplayıcı ekleri (docs/adr/0188): option A of each set, the owner's to change.
  calcObject: `<path d="M3 15C6 9 11 7 17 7"/><path d="M8.88 8.75 10.83 12.47" stroke-dasharray="2 1.5"/><circle cx="11.53" cy="13.79" r="1.5"/>${grip(3, 15)}`,
  calcKm: '<path d="M3 15h14M3 15v2.5M7.67 15v1.8M12.33 15v1.8M17 15v2.5"/><path d="M12.33 15V8.4" stroke-dasharray="2 1.5"/><circle cx="12.33" cy="6.8" r="1.6"/>',
  calcName: '<circle cx="5.5" cy="14.5" r="1.7"/><path d="M11.6 3.2 10.4 11.2M15.4 3.2l-1.2 8M9.2 5.8h7.6M8.8 8.6h7.6"/>',
  calcSlope: `<path d="M3 15 17 7"/><path d="M3 15h12.6"/><path d="M17 15V7" stroke-dasharray="2 1.5"/><circle cx="17" cy="15" r="1.5"/>${grip(3, 15)}`,
  calcBisector: `<path d="M17 16H3l5-13"/><path d="M3 16 12.56 9.43" stroke-dasharray="2 1.5"/><circle cx="13.88" cy="8.53" r="1.5"/>${grip(3, 16)}${grip(17, 16)}${grip(8, 3)}`,
  tracking: '<path d="M2.5 13.5h15M13.5 2.5v15" stroke-dasharray="2 1.7"/><path d="M4.5 11v5M2 13.5h5M11 3.5h5M13.5 1v5"/><circle cx="13.5" cy="13.5" r="1.4" fill="currentColor"/>',

  // App chrome
  fileNew: '<path d="M5 2.5h6.5L15 6v11.5H5z"/><path d="M11.5 2.5V6H15M10 9.5v5M7.5 12h5"/>',
  fileOpen: '<path d="M2.5 15.5v-11h5l1.5 2h7v2.5"/><path d="m2.5 15.5 2.4-7h12.6l-2.4 7z"/>',
  save: '<path d="M3.5 3.5h10l3 3v10h-13z"/><path d="M6.5 3.5v4h6v-4M6 16.5v-5h8v5"/>',
  cut: '<circle cx="6" cy="14.5" r="2.3"/><circle cx="14" cy="14.5" r="2.3"/><path d="M7.6 12.8 14 3.5M12.4 12.8 6 3.5"/>',
  paste: '<rect x="4.5" y="4" width="11" height="13.5" rx="1"/><path d="M7.5 4V2.8h5V4M7.5 9h5M7.5 12h5"/>',
  undo: '<path d="M7 4.8 3.5 8.3 7 11.8"/><path d="M3.5 8.3H12a4.4 4.4 0 0 1 0 8.8H8.5"/>',
  redo: '<path d="m13 4.8 3.5 3.5-3.5 3.5"/><path d="M16.5 8.3H8a4.4 4.4 0 0 0 0 8.8h3.5"/>',
  zoomIn: '<circle cx="8.5" cy="8.5" r="5.2"/><path d="m12.4 12.4 4.6 4.6M6.3 8.5h4.4M8.5 6.3v4.4"/>',
  zoomOut: '<circle cx="8.5" cy="8.5" r="5.2"/><path d="m12.4 12.4 4.6 4.6M6.3 8.5h4.4"/>',
  zoomExtents: '<path d="M3 7V3h4M13 3h4v4M17 13v4h-4M7 17H3v-4"/><rect x="7" y="7" width="6" height="6"/>',
  // Genel bakış and Büyüteç (docs/adr/0181): the whole drawing with the view's frame on it; a loupe with a reticle.
  overview: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><rect x="9.5" y="6" width="5.5" height="4.5"/><path d="m5 14 3-4 2.5 2.5"/>',
  magnifier: '<circle cx="8.5" cy="8.5" r="5.5"/><path d="M8.5 4.6v2.2M8.5 10.2v2.2M4.6 8.5h2.2M10.2 8.5h2.2M12.6 12.6l4.6 4.6"/>',
  zoomWindow: '<rect x="2.5" y="2.5" width="10" height="8" stroke-dasharray="2 1.6"/><circle cx="12" cy="12" r="3.2"/><path d="m14.4 14.4 3 3"/>',
  zoomSelection: '<path d="M10 2.5v3M10 14.5v3M2.5 10h3M14.5 10h3"/><circle cx="10" cy="10" r="4.5"/><circle cx="10" cy="10" r="1" fill="currentColor"/>',
  viewPrevious: '<path d="M3 7V3h4M13 3h4v4M17 13v4h-4M7 17H3v-4"/><path d="M11.5 6.5 8 10l3.5 3.5"/>',
  viewNext: '<path d="M3 7V3h4M13 3h4v4M17 13v4h-4M7 17H3v-4"/><path d="M8.5 6.5 12 10l-3.5 3.5"/>',
  extentCheck: '<path d="M3 7V3h4M13 3h4v4M17 13v4h-4M7 17H3v-4"/><rect x="5.5" y="5.5" width="5" height="5"/><circle cx="14.2" cy="14.2" r="1.4" fill="currentColor" stroke="none"/>',
  layers: '<path d="m10 3 7 3.8-7 3.8-7-3.8z"/><path d="m3 10.3 7 3.8 7-3.8"/><path d="m3 13.6 7 3.9 7-3.9"/>',
  layerAdd: '<path d="m9 3 6.5 3.5L9 10 2.5 6.5z"/><path d="m2.5 10 6.5 3.5 2-1.1M15.5 11.5v6M12.5 14.5h6"/>',
  // Yalnızca bunu göster: one layer, the others ghosted; Tüm katmanları göster: the layers under an eye.
  layerIsolate:
    '<path d="m10 3 7 3.8-7 3.8-7-3.8z" fill="currentColor" fill-opacity=".3"/><path d="m3 10.3 7 3.8 7-3.8M3 13.6l7 3.9 7-3.9" stroke-dasharray="1.6 1.6" stroke-width="1.1"/>',
  layersShowAll:
    '<path d="m10 8.6 7 3.3-7 3.3-7-3.3z"/><path d="m3 14.7 7 3.3 7-3.3"/><path d="M4.5 4.6S6.8 1.8 10 1.8s5.5 2.8 5.5 2.8-2.3 2.8-5.5 2.8-5.5-2.8-5.5-2.8z" stroke-width="1.2"/><circle cx="10" cy="4.6" r="1.2" fill="currentColor" stroke="none"/>',
  // The layer actions by an object (docs/adr/0177 §1): two plates under what is done to the layer.
  layerOff:
    '<path d="m3 12.2 7 3.4 7-3.4M3 15.2l7 3.4 7-3.4"/><path d="M4.4 6.4S6.6 3.4 10 3.4s5.6 3 5.6 3-2.2 3-5.6 3-5.6-3-5.6-3z" stroke-width="1.2"/><path d="M3.6 1.8 16.4 11" stroke-width="1.3"/>',
  layerLock:
    '<path d="m3 12.2 7 3.4 7-3.4M3 15.2l7 3.4 7-3.4"/><rect x="6.8" y="5.6" width="6.4" height="4.8" rx=".9" stroke-width="1.2"/><path d="M8.3 5.6V4.2a1.7 1.7 0 0 1 3.4 0v1.4" stroke-width="1.2"/>',
  layerMakeActive:
    '<path d="m10 7.4 7 3.6-7 3.6-7-3.6z" fill="currentColor" fill-opacity=".3"/><path d="m3 14.6 7 3.6 7-3.6"/><path d="m7 3.9 2.2 2.2 4.4-4.3" stroke-width="1.4"/>',
  // Katmanı eşle: an object carried onto the plates; Katmana kopyala: its copy carried, the object staying.
  layerMatch:
    '<path d="m3 12.2 7 3.4 7-3.4M3 15.2l7 3.4 7-3.4"/><rect x="2.8" y="2.4" width="5" height="5" rx=".6" fill="currentColor" fill-opacity=".3" stroke-width="1.2"/><path d="M9.2 4.9h4.6v4.6M12.1 7.8l1.7 1.7 1.7-1.7" stroke-width="1.2"/>',
  copyToLayer:
    '<path d="m3 12.2 7 3.4 7-3.4M3 15.2l7 3.4 7-3.4"/><rect x="2.4" y="2" width="4.4" height="4.4" rx=".6" stroke-width="1.1"/><rect x="4.4" y="4" width="4.4" height="4.4" rx=".6" fill="currentColor" fill-opacity=".3" stroke-width="1.1"/><path d="M10.2 6.2h3.6v3.6M12.1 8.1l1.7 1.7 1.7-1.7" stroke-width="1.2"/>',
  // Kopyasını oluştur: a layer and its copy beside it; Katmanları birleştir: two layers flowing into one.
  layerDuplicate:
    '<path d="m8 9.6 6 3.1-6 3.1-6-3.1z"/><path d="m12 4.2 6 3.1-6 3.1" fill="currentColor" fill-opacity=".3" stroke-dasharray="1.6 1.4"/><path d="M14.5 13.6v4.6M12.2 15.9h4.6" stroke-width="1.3"/>',
  layerMerge:
    '<path d="m10 12.4 7 3.6-7 3.6-7-3.6z" fill="currentColor" fill-opacity=".3"/><path d="M4.5 2.5v2.4A3 3 0 0 0 7.5 8H10m5.5-5.5v2.4a3 3 0 0 1-3 3.1H10v3.4m-1.6-1.6L10 11.4l1.6-1.6" stroke-width="1.3"/>',
  layerUnisolate:
    '<path d="m10 6.6 7 3.6-7 3.6-7-3.6z"/><path d="m3 13.8 7 3.6 7-3.6"/><path d="M13.6 4.2a4 4 0 0 0-6.9.6M6.4 2v2.8h2.8" stroke-width="1.2"/>',
  // Katman durumları (docs/adr/0177 §4): a bookmark over the plates, a new one with its plus; Kullanılmayanları temizle:
  // an empty plate swept; Katman listesi: the layers as rows.
  layerStates:
    '<path d="m3 12.2 7 3.4 7-3.4M3 15.2l7 3.4 7-3.4"/><path d="M7 2.2h6v8.4l-3-2.2-3 2.2z" fill="currentColor" fill-opacity=".3" stroke-width="1.2"/>',
  layerStateSave:
    '<path d="m3 12.2 7 3.4 7-3.4M3 15.2l7 3.4 7-3.4"/><path d="M4.4 2.2h5v7.6l-2.5-1.9-2.5 1.9z" stroke-width="1.2"/><path d="M14.2 2v6M11.2 5h6" stroke-width="1.3"/>',
  layerPurge:
    '<path d="m2.5 13.2 6.5 3.2 6.5-3.2-6.5-3.2z" stroke-dasharray="1.6 1.4"/><path d="m2.5 16.2 6.5 3.2 6.5-3.2"/><path d="M17.6 1.6 14.4 5.6" stroke-width="1.2"/><path d="M11.8 5.8h4.8l.9 3.6h-6.6z" stroke-width="1.2"/>',
  layerList:
    '<path d="m2.6 4.6 2.3-1.3 2.3 1.3-2.3 1.3zM2.6 10l2.3-1.3 2.3 1.3-2.3 1.3zM2.6 15.4l2.3-1.3 2.3 1.3-2.3 1.3z" stroke-width="1.1"/><path d="M9.5 4.6h8M9.5 10h8M9.5 15.4h8"/>',
  // Kayıtlı ölçüler (docs/adr/0180): a measured edge with its recorded tick and value mark; written from the drawing.
  cogoCheck:
    '<path d="M3 15.5 14.5 4" stroke-width="1.3"/><path d="M2 12.5l3 3M11.5 1.5l3 3" stroke-width="1.1"/><path d="m11.5 15 2 2 4-4.5" stroke-width="1.4"/>',
  cogoUpdate:
    '<path d="M3 15.5 14.5 4" stroke-width="1.3"/><path d="M2 12.5l3 3M11.5 1.5l3 3" stroke-width="1.1"/><path d="M15 10.5v7M12 14.5l3 3 3-3" stroke-width="1.3"/>',
  // Veri karşılaştır (docs/adr/0179): two shapes side by side, the second one changed, arrows between them.
  dataCompare:
    '<path d="M2.5 4.5h6v6h-6z" stroke-width="1.2"/><path d="M11.5 9.5h6v6h-4.2l-1.8-2.1z" fill="currentColor" fill-opacity=".3" stroke-width="1.2"/><path d="M5.5 13.5v3h3M14.5 6.5v-3h-3" stroke-width="1.1"/><path d="m7 15 1.5 1.5L7 18M13 2l-1.5 1.5L13 5" stroke-width="1.1"/>',
  folderAdd: '<path d="M2.5 15.5v-10h5l1.5 2h8.5v3"/><path d="M2.5 15.5h9M15 11.5v6M12 14.5h6"/>',
  folder: '<path d="M2.5 15.5v-10h5l1.5 2h8.5v8z"/>',
  eye: '<path d="M1.8 10S5 4.6 10 4.6 18.2 10 18.2 10 15 15.4 10 15.4 1.8 10 1.8 10z"/><circle cx="10" cy="10" r="2.4"/>',
  eyeOff: '<path d="M4.2 6.3C2.6 7.8 1.8 10 1.8 10S5 15.4 10 15.4c1.4 0 2.6-.4 3.7-1M8 4.8c.6-.1 1.3-.2 2-.2 5 0 8.2 5.4 8.2 5.4s-.7 1.3-2 2.6M3 3l14 14"/>',
  lock: '<rect x="4.5" y="9" width="11" height="8" rx="1.2"/><path d="M7 9V6.5a3 3 0 0 1 6 0V9"/>',
  unlock: '<rect x="4.5" y="9" width="11" height="8" rx="1.2"/><path d="M7 9V6.5a3 3 0 0 1 5.8-1.1"/>',
  // The digitizing locks (docs/adr/0166): what is held, a padlock in the corner.
  lockLength: `<path d="M2.5 8h11M2.5 5.5v5M13.5 5.5v5"/><path d="m5 6.5-1.5 1.5L5 9.5M11 6.5l1.5 1.5L11 9.5" stroke-width="1.1"/>${padlock(12.4, 13.3)}`,
  lockAngle: `<path d="M2.5 15.5h8.5M2.5 15.5 10.5 5"/><path d="M7.5 15.5a5 5 0 0 0-1.9-3.9"/>${padlock(12.4, 13.3)}`,
  lockDeflection: `<path d="M2.5 15 8.5 9h6.5"/><path d="M8.5 9 12 5.5" stroke-dasharray="1.5 1.4" stroke-width="1.1"/><path d="M10.6 6.9a3 3 0 0 1 .9 2.1" stroke-width="1.1"/>${padlock(12.4, 13.3)}`,
  // Nesneye paralel and dik: the picked edge, solid, and the locked direction through the reference, dashed.
  lockParallel: `<path d="M2.5 11.5 10 4"/><path d="M5 16.5 13 8.5" stroke-dasharray="1.8 1.5"/>${padlock(12.4, 13.3)}`,
  lockPerpendicular: `<path d="M2.5 15.5h9"/><path d="M6 15.5V3.5" stroke-dasharray="1.8 1.5"/><path d="M6 12.5h3v3" stroke-width="1.1"/>${padlock(12.4, 13.3)}`,
  // Dik açı: a path whose every turn is square, two of its corners marked.
  rightAngle: '<path d="M3 17V10h7V3.5h7"/><path d="M3 12.5h2.5V10M10 6h2.5V3.5" stroke-width="1.1"/>',
  // Referans noktası: a point marked R; Yapım kipi: construction lines through a point.
  lockReference: '<path d="M3.5 8.5l6 6M9.5 8.5l-6 6"/><path d="M12.5 3.5v7M12.5 3.5h2.4a1.8 1.8 0 0 1 0 3.6h-2.4M14.7 7.1l2.3 3.4" stroke-width="1.2"/>',
  lockConstruction: '<path d="M2.5 15.5 17.5 4.5M2.5 7l15 6.5" stroke-dasharray="1.8 1.5"/><circle cx="10" cy="10.2" r="1.9" fill="currentColor" stroke="none"/>',
  // Kalıcı: the padlock and a turning arrow, it stays for the next points.
  lockKeep:
    '<rect x="3" y="9.5" width="8.5" height="7" rx="1.1"/><path d="M5 9.5V7.6a2.25 2.25 0 0 1 4.5 0v1.9"/><path d="M14.2 7.2a3.8 3.8 0 1 1-.6 6.4"/><path d="m14.6 4.6-.4 2.7 2.7.3" stroke-width="1.2"/>',
  // A layer's own snapping (docs/adr/0163 §4): a horseshoe magnet, its poles marked; struck through when off.
  magnet: '<path d="M4.5 3.5h3.3v6.4a2.2 2.2 0 0 0 4.4 0V3.5h3.3v6.6a5.5 5.5 0 0 1-11 0z"/><path d="M4.5 6.8h3.3M12.2 6.8h3.3"/>',
  magnetOff: '<path d="M4.5 3.5h3.3v6.4a2.2 2.2 0 0 0 4.4 0V3.5h3.3v6.6a5.5 5.5 0 0 1-11 0z"/><path d="M4.5 6.8h3.3M12.2 6.8h3.3M3 3l14 14"/>',
  // Kinds of its own: the magnet dashed.
  magnetKinds: '<g stroke-dasharray="2.2 1.6"><path d="M4.5 3.5h3.3v6.4a2.2 2.2 0 0 0 4.4 0V3.5h3.3v6.6a5.5 5.5 0 0 1-11 0z"/></g><path d="M4.5 6.8h3.3M12.2 6.8h3.3"/>',
  chevronRight: '<path d="m8 5 5 5-5 5"/>',
  chevronDown: '<path d="m5 8 5 5 5-5"/>',
  chevronUp: '<path d="m5 12 5-5 5 5"/>',
  close: '<path d="m5 5 10 10M15 5 5 15"/>',
  check: '<path d="m4.5 10.5 3.5 3.5 7.5-8"/>',
  search: '<circle cx="8.5" cy="8.5" r="5"/><path d="m12.3 12.3 4.2 4.2"/>',
  // Veride ara (docs/adr/0178): the drawing's rows, a magnifier over them; İşareti kaldır: the target's mark struck out.
  dataSearch: '<path d="M2.5 4.5h9M2.5 8.5h5.5M2.5 12.5h4M2.5 16.5h4" stroke-width="1.2"/><circle cx="13" cy="11.5" r="3.6"/><path d="m15.7 14.2 2.3 2.4"/>',
  markClear: '<circle cx="8.5" cy="8.5" r="4.2"/><path d="M8.5 2v2.8M8.5 12.2V15M2 8.5h2.8M12.2 8.5H15"/><path d="m12.8 12.8 4.7 4.7m0-4.7-4.7 4.7" stroke-width="1.3"/>',
  grip: '<circle cx="7.5" cy="5" r="1" fill="currentColor" stroke="none"/><circle cx="12.5" cy="5" r="1" fill="currentColor" stroke="none"/><circle cx="7.5" cy="10" r="1" fill="currentColor" stroke="none"/><circle cx="12.5" cy="10" r="1" fill="currentColor" stroke="none"/><circle cx="7.5" cy="15" r="1" fill="currentColor" stroke="none"/><circle cx="12.5" cy="15" r="1" fill="currentColor" stroke="none"/>',
  columns: '<rect x="3.5" y="3.5" width="5" height="13" rx=".8"/><rect x="11.5" y="3.5" width="5" height="13" rx=".8"/>',
  dock: '<rect x="3" y="3.5" width="14" height="13" rx="1"/><path d="M7.5 3.5v13"/>',
  panelRight: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M12 3.5v13"/>',
  panelBottom: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 12h15"/>',
  toolbox: '<rect x="3" y="3" width="6" height="6" rx="1"/><rect x="11" y="3" width="6" height="6" rx="1"/><rect x="3" y="11" width="6" height="6" rx="1"/><rect x="11" y="11" width="6" height="6" rx="1"/>',
  terminal: '<rect x="2.5" y="4" width="15" height="12" rx="1"/><path d="m6 8 2.5 2L6 12M10.5 12.5h3.5"/>',
  snap: '<rect x="6.5" y="6.5" width="7" height="7"/><path d="M10 2v4.5M10 13.5V18M2 10h4.5M13.5 10H18"/>',
  // Sahneden seç (docs/adr/0088): KentOS UI's Icon::Target on this grid.
  target: '<circle cx="10" cy="10" r="5.9"/><path d="M10 1.9v3.4M10 14.7v3.4M1.9 10h3.4M14.7 10h3.4"/><circle cx="10" cy="10" r="1.3" fill="currentColor" stroke="none"/>',
  grid: '<path d="M3 7h14M3 13h14M7 3v14M13 3v14"/>',
  lineWeight: '<path d="M3 5h14"/><path d="M3 10h14" stroke-width="2.4"/><path d="M3 15.5h14" stroke-width="3.6"/>',
  // Görünüm kipleri (docs/adr/0195): Renkli, Tek renk, Gri; Dolgular, Alan sınırları, Saydamlık (the option sheet's A, Tek renk its B: A's half disc is Tema's).
  colorModeColor: '<circle cx="7.5" cy="8" r="4.5"/><circle cx="12.5" cy="8" r="4.5"/><circle cx="10" cy="12.5" r="4.5"/>',
  // Yazıların boyu (docs/adr/0205 §5): an A grown, an A on its ruler, an A on the screen.
  annotationLegible: '<path d="M2.5 15.5 6.5 4.5l4 11M4 11.8h5"/><path d="M12.5 11.5 15 9l2.5 2.5M15 9v7"/>',
  annotationTrue: '<path d="M3 12.5 7 2.5l4 10M4.5 8.8h5"/><path d="M2.5 16.5h15M5 15v1.5M10 15v1.5M15 15v1.5" stroke-width="1.2"/>',
  annotationScreen: '<rect x="2.5" y="3" width="15" height="10.5" rx="1.5"/><path d="M7 17h6M10 13.5V17"/><path d="M7.2 11.2 10 5.4l2.8 5.8M8.1 9.4h3.8" stroke-width="1.2"/>',
  colorModeMono: '<rect x="3" y="3" width="14" height="14" rx="2"/><path d="M3 17 17 3v12a2 2 0 0 1-2 2z" fill="currentColor" stroke="none"/>',
  colorModeGray: '<rect x="2.5" y="6.5" width="5" height="7" rx="0.8" fill="currentColor" stroke="none" opacity="0.9"/><rect x="7.5" y="6.5" width="5" height="7" rx="0.8" fill="currentColor" stroke="none" opacity="0.5"/><rect x="12.5" y="6.5" width="5" height="7" rx="0.8" fill="currentColor" stroke="none" opacity="0.2"/><rect x="2.5" y="6.5" width="15" height="7" rx="0.8"/>',
  viewFills: '<path d="M3 4.5h11l3 11H6z"/><path d="m6.5 8 2.5-3.5M7 12l5-7M9 15l5.5-8M12.5 15.2l3-4.2" stroke-width="1.1"/>',
  viewAreaEdges: '<path d="M3 4.5h11l3 11H6z" stroke-width="1.8"/><rect x="1.5" y="3" width="3" height="3" fill="currentColor" stroke="none"/><rect x="12.5" y="3" width="3" height="3" fill="currentColor" stroke="none"/><rect x="15.5" y="14" width="3" height="3" fill="currentColor" stroke="none"/><rect x="4.5" y="14" width="3" height="3" fill="currentColor" stroke="none"/>',
  viewTransparency: '<rect x="2.5" y="2.5" width="10" height="10" rx="1"/><rect x="7.5" y="7.5" width="10" height="10" rx="1" fill="currentColor" stroke="none" opacity="0.35"/><rect x="7.5" y="7.5" width="10" height="10" rx="1"/>',
  // A layer's colour (a drop, half full) and line type (solid, dashed, dash-dot).
  color: '<path d="M10 2.8C7.6 6 5 8.9 5 12a5 5 0 0 0 10 0c0-3.1-2.6-6-5-9.2z"/><path d="M5 12h10a5 5 0 0 1-10 0z" fill="currentColor" fill-opacity=".35" stroke="none"/>',
  lineType: '<path d="M3 5h14"/><path d="M3 10h14" stroke-dasharray="3 2.6"/><path d="M3 15h14" stroke-dasharray="5 2.6 .1 2.6"/>',
  // The project's plot scale (Özellikler's Ölçek): a scale bar, every other part filled.
  plotScale: '<rect x="2.5" y="7.5" width="15" height="5"/><path d="M2.5 7.5h3.75v5H2.5zM10 7.5h3.75v5H10z" fill="currentColor" stroke="none"/><path d="M2.5 15v1.5M10 15v1.5M17.5 15v1.5" stroke-width="1.2"/>',
  ortho: '<path d="M4 3.5v12.5h12.5"/><path d="M4 12h4v4"/>',
  polar: '<path d="M3 16.5h14M3 16.5 14.5 5"/><path d="M9 16.5a6 6 0 0 0-1.8-4.2"/>',
  // Çizim motoru: a chip, a triangle on it (WebGL2) or a bolt (WebGPU).
  rendererWebgl2:
    '<rect x="5" y="5" width="10" height="10" rx="1.5"/><path d="M8 2.5V5M12 2.5V5M8 15v2.5M12 15v2.5M2.5 8H5M2.5 12H5M15 8h2.5M15 12h2.5"/><path d="M10 7.6 12.6 12.2H7.4z" stroke-width="1.1"/>',
  rendererWebgpu:
    '<rect x="5" y="5" width="10" height="10" rx="1.5"/><path d="M8 2.5V5M12 2.5V5M8 15v2.5M12 15v2.5M2.5 8H5M2.5 12H5M15 8h2.5M15 12h2.5"/><path d="M10.8 7.2 8.6 10.4h2.8L9.2 13" stroke-width="1.2"/>',
  // Sembol boyutu: a symbol over a scale (the drawing's) or on a screen.
  symbolsPlot:
    '<path d="m10 4 1.25 2.55 2.8.4-2 1.98.47 2.8L10 10.4l-2.52 1.33.48-2.8-2-1.98 2.8-.4z" stroke-width="1.1"/><path d="M3 16h14" stroke-width="1.2"/><path d="M3 14v2M6.5 15v1M10 14v2M13.5 15v1M17 14v2" stroke-width="1"/>',
  symbolsScreen:
    '<rect x="2.5" y="3" width="15" height="10.5" rx="1" stroke-width="1.2"/><path d="M7 17h6M10 13.5V17" stroke-width="1.2"/><path d="m10 5 .95 1.95 2.15.3-1.55 1.5.37 2.14L10 9.88l-1.92 1.01.37-2.14-1.55-1.5 2.15-.3z" stroke-width="1"/>',
  // Son komutu yinele: going round again, played.
  repeat: '<path d="M15.6 10.2a5.6 5.6 0 1 1-1.7-4.1"/><path d="M14.6 2.6v3.7h-3.7"/><path d="M8.9 7.9v4.4l3.5-2.2z" fill="currentColor" stroke="none"/>',
  sun: '<circle cx="10" cy="10" r="3.4"/><path d="M10 2.5v1.8M10 15.7v1.8M2.5 10h1.8M15.7 10h1.8M4.7 4.7l1.3 1.3M14 14l1.3 1.3M4.7 15.3 6 14M14 6l1.3-1.3"/>',
  moon: '<path d="M15.8 12.6A6.5 6.5 0 0 1 7.4 4.2a6.5 6.5 0 1 0 8.4 8.4z"/>',
  keyboard: '<rect x="2" y="5" width="16" height="10" rx="1.2"/><path d="M5 8h1M8 8h1M11 8h1M14 8h1M5 11h1M14 11h1M8 11h4"/>',
  import: '<path d="M10 3v9M6.5 8.5 10 12l3.5-3.5"/><path d="M3.5 13v3.5h13V13"/>',
  export: '<path d="M10 12V3M6.5 6.5 10 3l3.5 3.5"/><path d="M3.5 13v3.5h13V13"/>',
  importDxf: fileIn(EMBLEM.dxf),
  importNcz: fileIn(EMBLEM.ncz),
  importShp: fileIn(EMBLEM.shp),
  importGeojson: fileIn(EMBLEM.geojson),
  importNcn: fileIn(EMBLEM.ncn),
  importKpafta: fileIn(EMBLEM.kpafta),
  importGnss: fileIn(EMBLEM.gnss),
  exportDxf: fileOut(EMBLEM.dxf),
  exportGeojson: fileOut(EMBLEM.geojson),
  exportNcn: fileOut(EMBLEM.ncn),
  exportPdf: fileOut(EMBLEM.pdf),
  exportSvg: fileOut(EMBLEM.svg),
  exportPng: fileOut(EMBLEM.png),
  exportKpafta: fileOut(EMBLEM.kpafta),
  print: '<path d="M5.5 7.5v-4h9v4"/><rect x="2.5" y="7.5" width="15" height="6.5" rx="1"/><path d="M5.5 12h9v5h-9z"/>',
  info: '<circle cx="10" cy="10" r="7"/><path d="M10 9v4.5M10 6.3v.2"/>',
  warning: '<path d="M10 3 17.5 16h-15z"/><path d="M10 8v3.8M10 13.8v.2"/>',
  error: '<circle cx="10" cy="10" r="7"/><path d="m7.3 7.3 5.4 5.4M12.7 7.3l-5.4 5.4"/>',
  success: '<circle cx="10" cy="10" r="7"/><path d="m6.8 10.2 2.3 2.3 4.2-4.6"/>',
  more: '<circle cx="5" cy="10" r="1" fill="currentColor"/><circle cx="10" cy="10" r="1" fill="currentColor"/><circle cx="15" cy="10" r="1" fill="currentColor"/>',
  // Project types (Proje türü)
  // A project's type (Proje türü ▾): two planes, a drawing's and a map's.
  projectType: `<path d="m10 10.2 7.5 3.4-7.5 3.4-7.5-3.4z" fill="currentColor" fill-opacity=".14"/><path d="M4.5 8.2 15.5 3.2"/>${grip(4.5, 8.2)}${grip(15.5, 3.2)}`,
  modeCad: `<path d="M3.5 16.5V4l12.5 12.5z"/><path d="M6.5 13.5V11.2l2.3 2.3z"/><path d="M3.5 7.5h1.6M3.5 10.5h1.6"/>`,
  modeGis: '<path d="M2.5 5.8 7 3.8l6 2 4.5-2v10.4L13 16.2l-6-2-4.5 2z" fill="currentColor" fill-opacity=".14"/><path d="M7 3.8v10.4M13 5.8v10.4"/>',
  modePlan3d: '<path d="m3 13.2 7 3.8 7-3.8"/><path d="M5 6.2 8.5 4.4 12 6.2 8.5 8z" fill="currentColor" fill-opacity=".14"/><path d="M5 6.2v6.4l3.5 1.9 3.5-1.9V6.2M8.5 8v6.5M12 9.6l2.8-1.4 2.2 1.1v3.6l-5 2.6"/>',
  modeDisaster: '<path d="M2 11h3l1.5-4 2.2 8.5 2.1-11 2 9.3 1.5-2.8H18"/>',
  fullscreen: '<path d="M3.5 7.5v-4h4M12.5 3.5h4v4M16.5 12.5v4h-4M7.5 16.5h-4v-4"/>',
  fullscreenExit: '<path d="M7.5 3.5v4h-4M16.5 7.5h-4v-4M12.5 16.5v-4h4M3.5 12.5h4v4"/>',
  crs: '<circle cx="10" cy="10" r="7"/><path d="M3 10h14M10 3c-2.5 2-2.5 12 0 14M10 3c2.5 2 2.5 12 0 14"/>',
  table: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.5h15M2.5 12h15M7.5 7.5v9"/>',
  // Tablo (docs/adr/0184): the table object's commands, a table with its badge; the editor's tools on a grid.
  tableInsert:
    '<rect x="2.5" y="3" width="11.5" height="3.5" fill="currentColor" fill-opacity=".3" stroke="none"/><rect x="2.5" y="3" width="11.5" height="10.5" rx="1"/><path d="M2.5 6.5h11.5M2.5 10h11.5M6.5 6.5v7"/><path d="M16.2 12.6v6M13.2 15.6h6" stroke-width="1.6"/>',
  tableEdit:
    '<rect x="2.5" y="3" width="11.5" height="10.5" rx="1"/><path d="M2.5 6.5h11.5M2.5 10h11.5M6.5 6.5v7"/><path d="m11.6 18.2.7-2.7 4.6-4.6 2 2-4.6 4.6z" stroke-width="1.2"/><path d="m15.6 12.2 2 2" stroke-width="1.1"/>',
  tableUpdate:
    '<rect x="2.5" y="3" width="11.5" height="10.5" rx="1"/><path d="M2.5 6.5h11.5M2.5 10h11.5M6.5 6.5v7"/><path d="M18.4 14.6a3 3 0 0 0-5.4-1.2M12.2 16.4a3 3 0 0 0 5.4 1.2" stroke-width="1.2"/><path d="M12.8 11.6v1.9h1.9M17.8 19.2v-1.9h-1.9" stroke-width="1.1"/>',
  // Tablo ekle's sources: a table with what it is made of (a file, points, areas, attributes).
  tableFile: '<rect x="2.5" y="3" width="11.5" height="10.5" rx="1"/><path d="M2.5 6.5h11.5M2.5 10h11.5M6.5 6.5v7"/><path d="M12.6 11.4h3.7l2.4 2.4v5.5h-6.1z" stroke-width="1.2"/><path d="M16.1 11.4v2.6h2.6M14.2 16.2h2.8M14.2 17.8h2" stroke-width="1"/>',
  tableCoordinates: '<rect x="2.5" y="3" width="11.5" height="10.5" rx="1"/><path d="M2.5 6.5h11.5M2.5 10h11.5M6.5 6.5v7"/><circle cx="15.8" cy="15.7" r="1.3" fill="currentColor" stroke="none"/><path d="M15.8 11.6v2.2M15.8 17.6v2M11.7 15.7h2.2M17.7 15.7h2" stroke-width="1.2"/>',
  tableAreas: '<rect x="2.5" y="3" width="11.5" height="10.5" rx="1"/><path d="M2.5 6.5h11.5M2.5 10h11.5M6.5 6.5v7"/><path d="m12.3 14.4 3.9-2.6 3.2 2.5-.9 4.5-4.8.6z" fill="currentColor" fill-opacity=".3" stroke-width="1.2"/>',
  tableAttributes: '<rect x="2.5" y="3" width="11.5" height="10.5" rx="1"/><path d="M2.5 6.5h11.5M2.5 10h11.5M6.5 6.5v7"/><path d="M12.2 12.2h3.9l3.3 3.3-3.9 3.9-3.3-3.3z" stroke-width="1.2"/><circle cx="14.3" cy="14.3" r=".8" fill="currentColor" stroke="none"/>',
  tableRowAbove: '<rect x="3" y="9" width="14" height="8.5" rx="1"/><path d="M3 13.2h14M10 9v8.5"/><path d="M10 1.8v5.4M7.3 4.5h5.4" stroke-width="1.6"/>',
  tableRowBelow: '<rect x="3" y="2.5" width="14" height="8.5" rx="1"/><path d="M3 6.8h14M10 2.5V11"/><path d="M10 12.8v5.4M7.3 15.5h5.4" stroke-width="1.6"/>',
  tableColumnLeft: '<rect x="9" y="3" width="8.5" height="14" rx="1"/><path d="M13.2 3v14M9 10h8.5"/><path d="M4.5 7.3v5.4M1.8 10h5.4" stroke-width="1.6"/>',
  tableColumnRight: '<rect x="2.5" y="3" width="8.5" height="14" rx="1"/><path d="M6.8 3v14M2.5 10H11"/><path d="M15.5 7.3v5.4M12.8 10h5.4" stroke-width="1.6"/>',
  tableRowDelete:
    '<rect x="2.5" y="3.5" width="11" height="13" rx="1"/><rect x="2.5" y="8" width="11" height="4" fill="currentColor" fill-opacity=".35" stroke="none"/><path d="M2.5 8h11M2.5 12h11M7 3.5v13"/><path d="M15 10h4" stroke-width="1.6"/>',
  tableColumnDelete:
    '<rect x="3.5" y="2.5" width="13" height="11" rx="1"/><rect x="8" y="2.5" width="4" height="11" fill="currentColor" fill-opacity=".35" stroke="none"/><path d="M8 2.5v11M12 2.5v11M3.5 7h13"/><path d="M8 17h4" stroke-width="1.6"/>',
  tableMerge: '<rect x="2.5" y="5" width="15" height="10" rx="1"/><path d="M10 5v2M10 13v2"/><path d="M4.4 10h4.1M7 8.5l1.5 1.5L7 11.5M15.6 10h-4.1M13 8.5 11.5 10l1.5 1.5" stroke-width="1.2"/>',
  tableUnmerge: '<rect x="2.5" y="5" width="15" height="10" rx="1"/><path d="M10 5v10"/><path d="M8.4 10H4.3M5.8 8.5 4.3 10l1.5 1.5M11.6 10h4.1M14.2 8.5l1.5 1.5-1.5 1.5" stroke-width="1.2"/>',
  tableHeader:
    '<rect x="2.5" y="3.5" width="15" height="4" fill="currentColor" fill-opacity=".45" stroke="none"/><rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.5h15M2.5 12h15M7.5 7.5v9M12.5 7.5v9"/>',
  tableFit: '<rect x="2.5" y="4.5" width="15" height="11" rx="1"/><path d="M6.5 4.5v11M13.5 4.5v11" stroke-dasharray="1.2 1.3" stroke-width="1"/><path d="M4.3 10h11.4M6.3 8 4.3 10l2 2M13.7 8l2 2-2 2" stroke-width="1.2"/>',
  tableGridAll: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.8h15M2.5 12.2h15M7.5 3.5v13M12.5 3.5v13"/>',
  tableGridOuter: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.8h15M2.5 12.2h15M7.5 3.5v13M12.5 3.5v13" stroke-dasharray="1 1.6" stroke-width=".9"/>',
  tableGridRows: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.8h15M2.5 12.2h15"/><path d="M7.5 3.5v13M12.5 3.5v13" stroke-dasharray="1 1.6" stroke-width=".9"/>',
  tableGridNone: '<path d="M4 5.6h4M11.5 5.6h4.5M4 10h3M11.5 10h3.5M4 14.4h4.5M11.5 14.4h2.5" stroke-width="1.3"/><rect x="2.5" y="3.5" width="15" height="13" rx="1" stroke-dasharray="1 1.6" stroke-width=".9"/>',
  tableFrame: '<rect x="3" y="4" width="14" height="12" rx=".6" stroke-width="2.6"/><path d="M3 8.2h14M3 11.8h14M8 4v12M12.5 4v12" stroke-width=".9"/>',
  cellLeft: '<path d="M3 4.5h14M3 8.2h9M3 11.8h14M3 15.5h9"/>',
  cellCenter: '<path d="M3 4.5h14M5.5 8.2h9M3 11.8h14M5.5 15.5h9"/>',
  cellRight: '<path d="M3 4.5h14M8 8.2h9M3 11.8h14M8 15.5h9"/>',
  // Processing: two steps joined by a flow, a third node waiting.
  processing: '<rect x="2.5" y="3" width="6" height="4.5" rx="1"/><circle cx="15" cy="5.25" r="2.25"/><rect x="11" y="12.5" width="6.5" height="4.5" rx="1"/><path d="M8.5 5.25h4.25M5.5 7.5v3.25a2 2 0 0 0 2 2H11"/><path d="m9.4 11 1.6 1.75-1.6 1.75"/>',
  numberVertices: `<path d="M3 13V4.5l6.5-2L13 6"/>${grip(3, 13)}${grip(3, 4.5)}${grip(9.5, 2.5)}<path d="M12 10v7.5M15.5 10v7.5M10.5 12.5H17M10.5 15H17"/>`,
  edgeLengths: `<path d="M3 16.5 10 3.5l7 13z"/><path d="M4.4 8.6 7.3 3.2M12.7 3.2l2.9 5.4M5.5 19h9" stroke-dasharray="1.6 1.4"/>`,
  selectExpression: '<path d="M3.5 3 12 8.2l-3.7 1-1.9 3.6z"/><path d="M11 13.2h6.5M11 16.4h6.5"/>',
  // Mekânsal ve öznitelik sorgusu (docs/adr/0200), the owner's choices (7 October): the select pointer with a parcel
  // holding an object; a parcel, its trees and a sigma; an arrow from a parcel's edge into its building; a sigma and
  // bars; a key going into a table.
  selectLocation:
    '<path d="M2 1.8 8.6 5.9l-2.9.8-1.5 2.8z"/><path d="M8.5 10 14.5 7.6l3.3 4.9-2.6 5.1H8.2z"/><rect x="11" y="11.3" width="3.4" height="3.4" fill="currentColor" fill-opacity=".28" stroke="none"/><rect x="11" y="11.3" width="3.4" height="3.4" stroke-width="1.1"/>',
  infoInside:
    '<path d="M2.5 9.2 7.5 6.6l5 1.9-1 8.9h-8z"/><circle cx="5.8" cy="11.6" r="1.1" fill="currentColor" stroke="none"/><circle cx="9" cy="10.9" r="1.1" fill="currentColor" stroke="none"/><circle cx="7.4" cy="14.4" r="1.1" fill="currentColor" stroke="none"/><path d="M18 2.8h-4.6l2.5 3-2.5 3H18" stroke-width="1.3"/>',
  infoEnclosing:
    '<path d="M2.5 6.5 9.5 2.8l8 3.6-1.5 11.1H4z"/><rect x="8" y="10.3" width="4.6" height="4.6" fill="currentColor" fill-opacity=".28" stroke="none"/><rect x="8" y="10.3" width="4.6" height="4.6" stroke-width="1.2"/><path d="M10.3 4.8v3.8M8.7 7.1l1.6 1.6 1.6-1.6" stroke-width="1.2"/>',
  statsSummary:
    '<path d="M9 3.2H2.8l3.4 4.6-3.4 4.6H9" stroke-width="1.4"/><path d="M11.8 17v-5M14.8 17V8.5M17.8 17v-6.5" stroke-width="2"/><path d="M10.3 17.2h8.5" stroke-width="1.1"/>',
  joinField:
    '<circle cx="5.2" cy="5.2" r="2.7"/><path d="m7.1 7.1 4 4M9.3 9.3l1.3-1.3M10.6 10.6l1.3-1.3" stroke-width="1.3"/><rect x="10.5" y="11" width="7.5" height="6.5" rx=".8"/><path d="M10.5 14.2h7.5M14.2 11v6.5" stroke-width="1.1"/>',
  // Geometri işlemleri (docs/adr/0201), the owner's choices (7 October): a path and its buffer's band; crop marks
  // about a clipped area; a 2 × 2 block, three of a group joined; Venn circles for Kesişim, Fark, Simetrik fark and
  // Birleşim, the result filled; a ring crossing itself under a magnifier; the ring fallen into its two triangles,
  // checked; a jagged outline and its simple area; a parcel moved from a skewed grid onto a square one.
  geoBuffer:
    '<path d="M4.2 14.6 9 7.6l6.8 3.4" stroke-width="7" stroke-opacity=".28"/><path d="M4.2 14.6 9 7.6l6.8 3.4" stroke-width="1.5"/><circle cx="4.2" cy="14.6" r="1.1" fill="currentColor" stroke="none"/><circle cx="15.8" cy="11" r="1.1" fill="currentColor" stroke="none"/>',
  geoClip:
    '<path d="M5.5 2.5v3h-3M14.5 2.5v3h3M5.5 17.5v-3h-3M14.5 17.5v-3h3" stroke-width="1.3"/><path d="M5.5 9.2 9.5 3.6l7 4.4-2.6 8.6H7.3z" stroke-width="1" stroke-dasharray="1.8 1.4"/><path d="M5.5 9.2 8.6 5.5h5.9v9H5.5z" fill="currentColor" fill-opacity=".5" stroke="none"/><path d="M5.5 9.2 8.6 5.5h5.9v9H5.5z" stroke-width="1.3"/>',
  geoDissolve:
    '<path d="M2.5 2.5h15v7.5h-7.5v7.5h-7.5z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M2.5 2.5h15v7.5h-7.5v7.5h-7.5z"/><path d="M10 2.5v7.5M2.5 10H10" stroke-width="1" stroke-dasharray="1.8 1.4"/><path d="M10 10h7.5v7.5H10z" stroke-width="1.1"/>',
  geoIntersection:
    '<circle cx="7.5" cy="10" r="5.5"/><circle cx="12.5" cy="10" r="5.5"/><path d="M10 5.1A5.5 5.5 0 0 1 10 14.9A5.5 5.5 0 0 1 10 5.1z" fill="currentColor" fill-opacity=".5" stroke="none"/>',
  geoDifference:
    '<circle cx="12.5" cy="10" r="5.5" stroke-width="1" stroke-dasharray="1.8 1.4"/><path d="M10 5.1A5.5 5.5 0 1 0 10 14.9A5.5 5.5 0 0 1 10 5.1z" fill="currentColor" fill-opacity=".5" stroke="none"/><path d="M10 5.1A5.5 5.5 0 1 0 10 14.9A5.5 5.5 0 0 1 10 5.1z"/>',
  geoSymDifference:
    '<circle cx="7.5" cy="10" r="5.5"/><circle cx="12.5" cy="10" r="5.5"/><path d="M10 5.1A5.5 5.5 0 1 0 10 14.9A5.5 5.5 0 0 1 10 5.1z" fill="currentColor" fill-opacity=".5" stroke="none"/><path d="M10 5.1A5.5 5.5 0 1 1 10 14.9A5.5 5.5 0 0 0 10 5.1z" fill="currentColor" fill-opacity=".5" stroke="none"/>',
  geoUnion:
    '<path d="M10 5.1A5.5 5.5 0 1 0 10 14.9A5.5 5.5 0 0 1 10 5.1z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M10 5.1A5.5 5.5 0 1 1 10 14.9A5.5 5.5 0 0 0 10 5.1z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M10 5.1A5.5 5.5 0 0 1 10 14.9A5.5 5.5 0 0 1 10 5.1z" fill="currentColor" fill-opacity=".28" stroke="none"/><circle cx="7.5" cy="10" r="5.5"/><circle cx="12.5" cy="10" r="5.5"/>',
  geoValidity:
    '<path d="M9.98 10.56 2.5 4.5V13L13 4.5v4.71" stroke-width="1.2"/><circle cx="12.8" cy="12.8" r="3.6"/><path d="m15.4 15.4 2.6 2.6" stroke-width="2"/><circle cx="12.8" cy="12.8" r="1" fill="currentColor" stroke="none"/>',
  geoRepair:
    '<path d="M2.5 3.5 8.6 9.5 2.5 15.5z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M2.5 3.5 8.6 9.5 2.5 15.5z"/><path d="M14.6 3.5 8.6 9.5l6 6z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M14.6 3.5 8.6 9.5l6 6z"/><path d="m13.5 16 1.8 1.8 3.2-3.6" stroke-width="1.5"/>',
  geoSimplify:
    '<path d="M3 15.5 4.6 4.4l10.6-.9 2.3 10.8z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M3 15.5 4.6 4.4l10.6-.9 2.3 10.8z"/><path d="M3 15.5 3.6 9.5l1.6-.9.7-3.6 2.6.9 1.9-2.4 2.9 1.3.8 2.9 2.4 1.4-.9 4.6-3.4.6-1.7 1.9z" stroke-width=".9" stroke-dasharray="1.8 1.4"/>',
  geoReproject:
    '<path d="M2.5 9.5 5 3.5h6l-2.5 6z" stroke-width="1.1"/><path d="M3.75 6.5h6M8 3.5l-2.5 6" stroke-width=".9"/><path d="M11.5 11h6v6.5h-6z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M11.5 11h6v6.5h-6z"/><path d="M8 12.5h2.5M9.2 11.2l1.3 1.3-1.3 1.3" stroke-width="1.2"/>',
  // Topoloji kuralları (docs/adr/0202; the owner's choices, all A): Topolojiyi denetle and the Topoloji tab, three parcels
  // of a block with a check on their corner; Topoloji kuralları, a small block and the rules as lines; İstisna, a
  // flag; Düzelt, a parcel's corner and a wrench.
  topologyCheck:
    '<path d="M2.5 2.5h6.5v7h-6.5zM9 2.5h6.5v5.2M2.5 9.5v6h6" stroke-width="1.3"/><circle cx="14" cy="14" r="4.2" stroke-width="1.3"/><path d="m12 14.1 1.4 1.4 2.7-2.9" stroke-width="1.4"/>',
  topologyRules:
    '<path d="M2.5 2.5h7v6h-7zM6 2.5v6" stroke-width="1.2"/><path d="M8 12h9.5M8 15.5h9.5M2.5 12h2M2.5 15.5h2" stroke-width="1.3"/><path d="M12 5.5h5.5" stroke-width="1.3"/>',
  topologyException:
    '<path d="M5 17.5V2.5" stroke-width="1.5"/><path d="M5 3h10.5l-2.4 3.6 2.4 3.6H5z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="M5 3h10.5l-2.4 3.6 2.4 3.6H5z" stroke-width="1.3"/>',
  topologyFix:
    '<path d="M2.5 17.5V8.5h6" stroke-width="1.3"/><path d="M2.5 17.5h9" stroke-width="1.3"/><path d="M17.3 5.6a3 3 0 0 1-3.9 3L7 15l-1.6-1.6 6.4-6.4a3 3 0 0 1 3-3.9l-1.8 1.8.4 1.6 1.6.4z" stroke-width="1.2"/>',
  // Ağ analizi (docs/adr/0209 §10; chosen without asking, as the owner said): Ağlar, a graph of nodes and edges; En kısa
  // yol, a street grid with the route along it between its two stops; Hizmet alanı, a facility and the two bands it
  // reaches; Şebeke izleme, a pipe and its branch traced up to a valve. İşlemler's: En yakın tesis, an incident and two
  // facilities, the nearer one's way bold; Maliyet matrisi, every origin to every destination; Hizmet alanları, two
  // facilities' overlapping areas.
  networks:
    '<path d="M3.5 15.5 8 4.5l8.5 1.5L13 15.5zM8 4.5l5 11" stroke-width="1.2"/><circle cx="3.5" cy="15.5" r="1.9" fill="currentColor" stroke="none"/><circle cx="8" cy="4.5" r="1.9" fill="currentColor" stroke="none"/><circle cx="16.5" cy="6" r="1.9" fill="currentColor" stroke="none"/><circle cx="13" cy="15.5" r="1.9" fill="currentColor" stroke="none"/>',
  netRoute:
    '<path d="M2 6h16M2 14h16M6 2v16M14 2v16" stroke-width=".9" stroke-opacity=".5"/><path d="M3.5 14H14V4.5" stroke-width="2.2"/><circle cx="3.5" cy="14" r="2" fill="currentColor" stroke="none"/><path d="M14 4.5V1.8l3.2 1.3L14 4.4" fill="currentColor" stroke-width="1"/>',
  netServiceArea:
    '<path d="M10 2.2 15.6 4.8 18 10.4 14.6 16.9 8.2 18 2.6 13.2 2.8 6.6z" fill="currentColor" fill-opacity=".18" stroke-width="1.1"/><path d="M10 6.2 13.5 8.2 14 11.6 11.4 14.1 7.5 13.6 5.9 10.3 7.2 7.3z" fill="currentColor" fill-opacity=".38" stroke-width="1.1"/><circle cx="10" cy="10.2" r="1.7" fill="currentColor" stroke="none"/>',
  netTrace:
    '<path d="M15.5 10H18" stroke-width="1.2"/><path d="M2.5 10H12M7 10v7.5" stroke-width="2.4"/><path d="M12 7.8v4.4l3.5-4.4v4.4z" fill="currentColor" fill-opacity=".28" stroke-width="1.1"/><circle cx="2.8" cy="10" r="1.8" fill="currentColor" stroke="none"/>',
  closestFacility:
    '<path d="M4 16h5V5h5.5" stroke-width="2.2"/><path d="M9 16h7.5" stroke-width="1" stroke-dasharray="1.8 1.4"/><circle cx="4" cy="16" r="2" fill="currentColor" stroke="none"/><rect x="14.5" y="2.8" width="4" height="4" fill="currentColor" fill-opacity=".28" stroke-width="1.1"/><rect x="16.5" y="14" width="3" height="3.6" fill="currentColor" fill-opacity=".28" stroke-width="1"/>',
  odMatrix:
    '<path d="M4 4 16 6M4 4l12 9M4 10l12-4M4 10l12 3M4 16 16 6M4 16l12-3" stroke-width=".9"/><circle cx="4" cy="4" r="1.8" fill="currentColor" stroke="none"/><circle cx="4" cy="10" r="1.8" fill="currentColor" stroke="none"/><circle cx="4" cy="16" r="1.8" fill="currentColor" stroke="none"/><rect x="14.5" y="4.5" width="3.2" height="3.2" fill="currentColor" fill-opacity=".28" stroke-width="1.1"/><rect x="14.5" y="11.4" width="3.2" height="3.2" fill="currentColor" fill-opacity=".28" stroke-width="1.1"/>',
  serviceAreas:
    '<path d="M6.8 3.2 11 5.4 12 10.2 9.4 14.4 4.4 14.6 1.8 10.2 3 5.6z" fill="currentColor" fill-opacity=".22" stroke-width="1.1"/><path d="M13.4 6.6 17.6 8.4 18.4 13.4 15.6 17.4 10.6 17 8.4 12.8 10.2 8.2z" fill="currentColor" fill-opacity=".22" stroke-width="1.1"/><circle cx="6.6" cy="9.4" r="1.5" fill="currentColor" stroke="none"/><circle cx="13.6" cy="12.4" r="1.5" fill="currentColor" stroke="none"/>',
  // Öznitelik tablosu (docs/adr/0199 §4): a table and its layer's object (the owner's choice A).
  featureTable:
    '<rect x="2" y="2.5" width="11.5" height="10" rx="1"/><path d="M2 6h11.5M2 9.25h11.5M6 6v6.5"/><path d="m12 13.4 3.6-2.4 3 2.4-1.1 5h-4.6z" fill="currentColor" fill-opacity=".28" stroke="none"/><path d="m12 13.4 3.6-2.4 3 2.4-1.1 5h-4.6z"/>',
  // Alanlar (docs/adr/0199 §3): a form of labelled boxes, the last a list (the owner's choice B).
  layerFields:
    '<path d="M2.5 4h4M2.5 9h4M2.5 14h4" stroke-width="1.2"/><rect x="8.5" y="2.5" width="9" height="3" rx=".8"/><rect x="8.5" y="7.5" width="9" height="3" rx=".8"/><rect x="8.5" y="12.5" width="9" height="3" rx=".8"/><path d="m15 13.5.9.9.9-.9" stroke-width="1"/>',
  // Verilerden al (docs/adr/0199 §3): the rows' keys go up into the header (the owner's choice A).
  fieldsFromData:
    '<rect x="2.5" y="3" width="15" height="4" fill="currentColor" fill-opacity=".28" stroke="none"/><rect x="2.5" y="3" width="15" height="14" rx="1"/><path d="M2.5 7h15M5 11h3M5 14.5h3"/><path d="M13 15.5v-6M10.8 11.6 13 9.4l2.2 2.2" stroke-width="1.3"/>',
  // Veri kaynakları (docs/adr/0199 §7): folders and KentOS's projects (the owner's choice A).
  dataSources:
    '<path d="M2 15.5v-9h4.2l1.3 1.6H11" stroke-width="1.3"/><path d="M2 15.5h7"/><ellipse cx="14.5" cy="5.5" rx="3.5" ry="1.6"/><path d="M11 5.5v10c0 .9 1.6 1.6 3.5 1.6s3.5-.7 3.5-1.6v-10M11 10.5c0 .9 1.6 1.6 3.5 1.6s3.5-.7 3.5-1.6"/>',
  fieldCalc: '<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.5h15M8 7.5v9"/><path d="M11 12h4M13 10v4"/>',
  // The expression builder: ε, as QGIS marks it.
  expression: '<path d="M13.6 6.1A4.9 4.9 0 0 0 10 4.6c-2.3 0-3.8 1.2-3.8 2.8 0 1.6 1.4 2.6 3.3 2.6h1.8M9.5 10c-2.2 0-3.8 1.2-3.8 2.9 0 1.7 1.6 2.9 4 2.9 1.5 0 2.8-.5 3.9-1.4"/>',
  modelNew: '<rect x="2.5" y="3" width="6" height="4.5" rx="1"/><rect x="11" y="12.5" width="6.5" height="4.5" rx="1"/><path d="M5.5 7.5v3.25a2 2 0 0 0 2 2H11"/><path d="M14.25 3.5v5M11.75 6h5"/>',
  edit: '<path d="M4 16l.9-3.6 8.4-8.4a1.8 1.8 0 0 1 2.7 2.7l-8.4 8.4z"/><path d="M12 5.3l2.7 2.7"/>',
  play: '<path d="M6.5 4.5v11l8.5-5.5z"/>',
  history: '<path d="M3.5 10a6.5 6.5 0 1 0 2-4.7"/><path d="M3 3.3v3h3M10 6.5V10l2.5 1.8"/>',
  clear: '<path d="M4 6h12M8 6V4h4v2M5.5 6l.8 10.5h7.4L14.5 6"/>',
  settings: '<path d="M3.5 6h8M15.5 6h1M3.5 14h1M8.5 14h8"/><circle cx="13.5" cy="6" r="2"/><circle cx="6.5" cy="14" r="2"/>',
  units: '<path d="M3.5 16.5v-13l13 13z"/><path d="M3.5 12.5h2M3.5 9h2M7.5 16.5v-2M11 16.5v-2"/><path d="M6.5 13.5v-3l3 3z"/>',
  chip: '<rect x="5" y="5" width="10" height="10" rx="1.5"/><rect x="8" y="8" width="4" height="4"/><path d="M8 2.5V5M12 2.5V5M8 15v2.5M12 15v2.5M2.5 8H5M2.5 12H5M15 8h2.5M15 12h2.5"/>',
  appearance: '<circle cx="10" cy="10" r="6.5"/><path d="M10 3.5a6.5 6.5 0 0 1 0 13z" fill="currentColor" stroke="none"/>',
  // Yazı stilleri and Ölçü stilleri (docs/adr/0183 §5): a text's T, a dimension's line, each beside the sliders of its settings.
  textStyle: '<path d="M2.5 4.5h8.5M6.75 4.5v11"/><path d="M13 6h4.5M13 10h4.5M13 14h4.5"/><circle cx="14.6" cy="6" r="1.1" fill="currentColor"/><circle cx="16.2" cy="10" r="1.1" fill="currentColor"/><circle cx="14.2" cy="14" r="1.1" fill="currentColor"/>',
  dimensionStyle: '<path d="M2.5 6v9M10.5 6v9M2.5 12.5h8"/><path d="m4.3 11.3-1.8 1.2 1.8 1.2M8.7 11.3l1.8 1.2-1.8 1.2"/><path d="M13 6h4.5M13 10h4.5M13 14h4.5"/><circle cx="14.6" cy="6" r="1.1" fill="currentColor"/><circle cx="16.2" cy="10" r="1.1" fill="currentColor"/><circle cx="14.2" cy="14" r="1.1" fill="currentColor"/>',
  styles: '<path d="M10 3a7 7 0 1 0 0 14c1 0 1.5-.8 1.2-1.6-.4-1 .2-2 1.3-2H14a3.5 3.5 0 0 0 3.5-3.5C17.5 6 14.1 3 10 3z"/><circle cx="6.6" cy="9.2" r=".9"/><circle cx="8.9" cy="6.2" r=".9"/><circle cx="12.6" cy="6.6" r=".9"/>',
  layerStyle: '<rect x="3" y="3" width="10" height="10" rx="1"/><path d="m3 8.5 5.5-5.5M5 13l8-8"/><circle cx="14.5" cy="14.5" r="3"/>',
  // Commands shown as ribbon buttons.
  saveAs: '<path d="M3.5 3.5h10l3 3v4"/><path d="M3.5 3.5v13h6"/><path d="M6.5 3.5v4h6v-4M6 16.5v-5h5"/><path d="m12.4 17.6.6-2.6 4.1-4.1a1.2 1.2 0 0 1 1.7 1.7L14.7 16.7z"/>',
  pasteOriginal: `<rect x="3.5" y="4" width="10" height="13.5" rx="1"/><path d="M6.3 4V2.8h4.4V4"/><path d="M15.5 9.5v7M12 13h7"/><circle cx="15.5" cy="13" r="2" fill="none"/>`,
  selectAll: '<rect x="3" y="3" width="14" height="14" stroke-dasharray="2 1.6"/><path d="M7 6.5 13 10l-2.6.7-1.3 2.5z"/>',
  deselect: '<rect x="3" y="3" width="14" height="14" stroke-dasharray="2 1.6"/><path d="m7.5 7.5 5 5M12.5 7.5l-5 5"/>',
  invertSelection: '<rect x="3" y="3" width="14" height="14" stroke-dasharray="2 1.6"/><path d="M10 3v14"/><path d="M10 3h7v14h-7z" fill="currentColor" fill-opacity=".3" stroke="none"/>',
  contours: '<path d="M2.5 13.5c2-3.2 4.3-2 6.3-4.3 2.2-2.5 4.2-4.8 8.7-3.7"/><path d="M4.2 16.5c2.3-2.4 4.7-1.5 6.7-3.5 1.8-1.8 3.4-3.4 6.6-2.8"/><path d="M2.5 9.6c1.8-2.4 3.8-2 5.3-3.8 1.3-1.6 2.6-2.8 4.9-2.8"/>',
  profile: '<path d="M3 3v14h14"/><path d="m5 13 3-4 2.5 2 3-5.5 2.5 3.5"/><path d="M5 17v-1.2M10.5 17v-1.2M15.5 17v-1.2"/>',
  sheet: '<rect x="3" y="3.5" width="14" height="13"/><path d="M3 7.8h14M3 12.2h14M7.7 3.5v13M12.3 3.5v13" stroke-dasharray="1.6 1.4"/>',
  parcelReport: '<path d="M4.5 2.5h8l3 3v12h-11z"/><path d="M12.5 2.5v3h3"/><path d="M7 9.5h6M7 12.5h6M7 15.5h3.5"/><path d="M10 9.5v6" stroke-dasharray="1.2 1.2"/>',
  crsTransform: '<circle cx="7" cy="7" r="4.5"/><path d="M2.5 7h9M7 2.5c-1.6 1.3-1.6 7.7 0 9"/><path d="M11.5 13.5h6M15.5 11.5l2 2-2 2M17.5 17h-6M13.5 15l-2 2 2 2"/>',
  crsQuery: '<path d="M10 2.5v4M10 13.5v4M2.5 10h4M13.5 10h4"/><circle cx="10" cy="10" r="4.5"/><circle cx="10" cy="10" r=".9" fill="currentColor" stroke="none"/>',
  // The project's second coordinate system (docs/adr/0167 §1): a globe and a small 2.
  crsSecond: '<circle cx="8.5" cy="11.5" r="5.5"/><path d="M3 11.5h11M8.5 6c-2 1.6-2 9.4 0 11M8.5 6c2 1.6 2 9.4 0 11"/><path d="M13.8 4.5a1.9 1.9 0 0 1 3.7.4c0 1.2-1.2 1.9-3.7 3.4h3.9"/>',
  volume: '<path d="m10 2.8 6.8 3.7v7L10 17.2l-6.8-3.7v-7z"/><path d="m3.2 6.5 6.8 3.7 6.8-3.7M10 10.2v7"/><path d="m10 10.2 6.8-3.7v7L10 17.2z" fill="currentColor" fill-opacity=".18" stroke="none"/>',
  slope: '<path d="M2.5 16.5h15L17.5 5.5z" fill="currentColor" fill-opacity=".14"/><path d="M2.5 16.5h15V5.5z"/><path d="M8 16.5a5.6 5.6 0 0 0-.9-3.1"/>',
  // Yüzey analizi (docs/adr/0231 §10): Bakı a compass rose, north marked, the way down; Gölgeli kabartma a hill lit from
  // the north-west, its far side in shade; Renkli kabartma a hill in colour bands; Eğrilik a convex and a concave
  // section over its datum, the hump's circle; Pürüzlülük broken ground over its datum; Güneşlenme the sun's rays on a
  // sloping face.
  aspect:
    '<circle cx="10" cy="10.5" r="7"/><path d="m10 1.6 1.5 3h-3z" fill="currentColor" stroke="none"/><path d="M17 10.5h1.6M1.4 10.5H3M10 17.5v1.6" stroke-width="1.1"/><path d="M10 10.5l3.4 3.4" stroke-width="1.5"/><path d="m15.5 16-4.6-1.1 3.5-3.5z" fill="currentColor" stroke="none"/><circle cx="10" cy="10.5" r="1" fill="currentColor" stroke="none"/>',
  hillshade:
    '<path d="M1.5 17 9.5 5.5 18.5 17z"/><path d="M9.5 5.5 18.5 17h-6.8l-1.4-5.2z" fill="currentColor" fill-opacity=".5" stroke="none"/><circle cx="3.6" cy="3.6" r="1.5" stroke-width="1.1"/><path d="M5.6 5.6l1.3 1.3" stroke-width="1.1" stroke-dasharray="1 .9"/>',
  colorRelief:
    '<path d="M2 16.5h16l-2.65-4.3H4.65z" fill="currentColor" fill-opacity=".2" stroke="none"/><path d="M4.65 12.2h10.7l-2.64-4.3H7.29z" fill="currentColor" fill-opacity=".5" stroke="none"/><path d="M7.29 7.9h5.42L10 3.5z" fill="currentColor" fill-opacity=".85" stroke="none"/><path d="M2 16.5 10 3.5l8 13z"/><path d="M4.65 12.2h10.7M7.29 7.9h5.42" stroke-width=".9"/>',
  curvature:
    '<path d="M1.5 12.5c2.4-7.4 6.4-7.4 8.5-2.6s6.1 4.8 8.5-2.6"/><circle cx="6.2" cy="9.9" r="2.6" stroke-width="1" stroke-dasharray="1.3 1.1"/><circle cx="13.9" cy="11.1" r="2.3" stroke-width="1" stroke-dasharray="1.3 1.1"/><path d="M1.5 17.5h17" stroke-width="1"/>',
  ruggedness:
    '<path d="M1.5 14.5 3.8 8l2.1 4.2L8.4 4.5l2.2 7.3 2.1-4.4 2.3 6.1 1.6-3.4 1.9 2.4"/><path d="M1.5 17.5h17" stroke-width="1"/><path d="M1.5 14.5 3.8 8l2.1 4.2L8.4 4.5l2.2 7.3 2.1-4.4 2.3 6.1 1.6-3.4 1.9 2.4v3.4h-17z" fill="currentColor" fill-opacity=".16" stroke="none"/>',
  insolation:
    '<circle cx="5" cy="5" r="2.3"/><path d="M5 .9v1.2M5 7.9v1.2M.9 5h1.2M7.9 5h1.2M2.1 2.1l.8.8M7.1 7.1l.8.8M7.9 2.1l-.8.8M2.9 7.1l-.8.8" stroke-width="1.1"/><path d="M3 17.5h15.5V9z" fill="currentColor" fill-opacity=".22"/><path d="M8.6 8.6l3.6 3.6M12.2 6.6l3.1 3.1" stroke-width="1.1" stroke-dasharray="1.5 1.1"/>',
  // İnterpolasyon and Yoğunluk (docs/adr/0232).
  idw: '<circle cx="10" cy="10" r="1.7" fill="currentColor" stroke="none"/><circle cx="4.2" cy="5" r="1.4"/><circle cx="16" cy="6.5" r="1.4"/><circle cx="5.5" cy="16" r="1.4"/><circle cx="16.8" cy="16.6" r="1.4"/><path d="M8.6 8.8 5.3 5.9M11.5 9.1l3.3-1.8" stroke-width="1.3"/><path d="M8.9 11.3l-2.4 3.4" stroke-width="1.1" stroke-opacity=".7"/><path d="M11.3 11.3l4.4 4.3" stroke-width=".9" stroke-opacity=".45" stroke-dasharray="1.3 1"/>',
  naturalNeighbor:
    '<path d="M10 5.3l4.2 2.5v4.9L10 15.1l-4.2-2.4V7.8z" fill="currentColor" fill-opacity=".22"/><path d="M10 5.3V1.5M14.2 7.8l3.9-2.3M14.2 12.7l3.9 2.4M10 15.1v3.4M5.8 12.7l-3.9 2.4M5.8 7.8 1.9 5.5" stroke-width="1.1"/><circle cx="10" cy="10.2" r="1.3" fill="currentColor" stroke="none"/><circle cx="4.2" cy="2.8" r=".9" fill="currentColor" stroke="none"/><circle cx="17.2" cy="10.2" r=".9" fill="currentColor" stroke="none"/><circle cx="4.4" cy="16.9" r=".9" fill="currentColor" stroke="none"/>',
  splineSurface:
    '<path d="M2 12.5c3-5.2 5.6-5.4 8-1.8s5 4.2 8-2.7"/><path d="M2 17c3-4.6 5.6-4.8 8-1.6s5 3.7 8-2.4" stroke-opacity=".45"/><circle cx="2.6" cy="11.6" r="1.3" fill="currentColor" stroke="none"/><circle cx="7.4" cy="8.2" r="1.3" fill="currentColor" stroke="none"/><circle cx="13" cy="13.2" r="1.3" fill="currentColor" stroke="none"/><circle cx="17.4" cy="8.9" r="1.3" fill="currentColor" stroke="none"/>',
  kriging:
    '<path d="M2.5 2v15.5H18" stroke-width="1.2"/><path d="M3 16.5c2.4-6.6 4.9-9.8 8.4-10.6 1.6-.4 3.6-.4 6.1-.4"/><circle cx="5.3" cy="11.8" r="1" fill="currentColor" stroke="none"/><circle cx="7.9" cy="9.4" r="1" fill="currentColor" stroke="none"/><circle cx="10.6" cy="6.9" r="1" fill="currentColor" stroke="none"/><circle cx="13.6" cy="6.1" r="1" fill="currentColor" stroke="none"/><circle cx="16.4" cy="5" r="1" fill="currentColor" stroke="none"/>',
  tinRaster:
    '<path d="M2 7.5h16M2 13h16M7.5 2v16M13 2v16" stroke-width=".7" stroke-opacity=".35"/><path d="M2.5 16.5 7.5 3.2 17.5 7 14.2 17zM7.5 3.2l6.7 13.8M2.5 16.5 17.5 7"/>',
  kernelDensity:
    '<circle cx="8" cy="8.5" r="6" fill="currentColor" fill-opacity=".13" stroke="none"/><circle cx="8" cy="8.5" r="3.8" fill="currentColor" fill-opacity=".25" stroke="none"/><circle cx="8" cy="8.5" r="1.7" fill="currentColor" stroke="none"/><circle cx="14.6" cy="14.4" r="3.6" fill="currentColor" fill-opacity=".18" stroke="none"/><circle cx="14.6" cy="14.4" r="1.4" fill="currentColor" stroke="none"/><circle cx="8" cy="8.5" r="6" stroke-width=".8" stroke-dasharray="1.2 1"/>',
  lineDensity:
    '<circle cx="10" cy="10" r="5.6" stroke-width="1" stroke-dasharray="1.4 1.1"/><path d="M1.5 7.2 18.5 13M5 18.5 12.4 1.5M1.5 14.5 18.5 4.6" stroke-opacity=".45"/><path d="M4.7 8.3l10.6 3.6M7.7 13.8l4.6-10.6" stroke-width="2"/>',
  // Raster işlemleri and Raster istatistiği (docs/adr/0233): Raster hesaplayıcı a calculator whose keys are a raster's cells;
  // Yeniden sınıflandır a column of shades turned into two classes; Maskeyle kırp a raster cut by an area, the cells outside
  // faded; Mozaik two sheets joined over their overlap; Yeniden örnekle a fine grid into a coarse one; Bölgesel istatistik
  // an area holding its figures' bars; Histogram bars on a base line; Komşuluk istatistiği a 3 × 3 window round a cell on a
  // grid; Hücre istatistiği a stack of rasters and one cell's column through them.
  rasterCalculator:
    '<rect x="3.5" y="1.8" width="13" height="16.4" rx="1.6"/><rect x="5.8" y="4" width="8.4" height="3.3" rx=".5" stroke-width="1"/><rect x="5.8" y="9.4" width="2.4" height="2.4" fill="currentColor" stroke="none"/><rect x="8.8" y="9.4" width="2.4" height="2.4" fill="currentColor" fill-opacity=".45" stroke="none"/><rect x="11.8" y="9.4" width="2.4" height="2.4" fill="currentColor" fill-opacity=".2" stroke="none"/><rect x="5.8" y="13.2" width="2.4" height="2.4" fill="currentColor" fill-opacity=".2" stroke="none"/><rect x="8.8" y="13.2" width="2.4" height="2.4" fill="currentColor" stroke="none"/><rect x="11.8" y="13.2" width="2.4" height="2.4" fill="currentColor" fill-opacity=".45" stroke="none"/>',
  reclassify:
    '<rect x="1.8" y="2.5" width="5" height="15" rx=".6" stroke-width="1.1"/><rect x="1.8" y="2.5" width="5" height="3.75" fill="currentColor" fill-opacity=".15" stroke="none"/><rect x="1.8" y="6.25" width="5" height="3.75" fill="currentColor" fill-opacity=".38" stroke="none"/><rect x="1.8" y="10" width="5" height="3.75" fill="currentColor" fill-opacity=".62" stroke="none"/><rect x="1.8" y="13.75" width="5" height="3.75" fill="currentColor" fill-opacity=".9" stroke="none"/><path d="M8.6 10h3.6M10.6 8.2l1.8 1.8-1.8 1.8" stroke-width="1.2"/><rect x="13.6" y="2.5" width="4.6" height="15" rx=".6" stroke-width="1.1"/><rect x="13.6" y="10" width="4.6" height="7.5" fill="currentColor" stroke="none"/>',
  clipRaster:
    '<path d="M2 7h16M2 12.5h16M7.3 2v16M12.7 2v16" stroke-width=".8" stroke-opacity=".35"/><rect x="2" y="2" width="16" height="16" rx="1" stroke-width="1" stroke-opacity=".35"/><path d="M5 5.5 14.8 4l1.6 8.5-6.4 4.8-5.6-3.6z" fill="currentColor" fill-opacity=".2" stroke-width="1.5"/>',
  mosaic:
    '<rect x="2" y="2" width="10.5" height="10.5" rx=".8"/><rect x="7.5" y="7.5" width="10.5" height="10.5" rx=".8"/><rect x="7.5" y="7.5" width="5" height="5" fill="currentColor" fill-opacity=".45" stroke="none"/><path d="M2 5.5h10.5M5.5 2v10.5M7.5 15h10.5M15 7.5v10.5" stroke-width=".8" stroke-opacity=".45"/>',
  resample:
    '<rect x="2" y="2" width="16" height="16" rx="1"/><path d="M10 2v16M2 10h16" stroke-width="1.3"/><path d="M4.67 2v8M7.33 2v8M2 4.67h8M2 7.33h8" stroke-width=".7"/><rect x="10" y="10" width="8" height="8" fill="currentColor" fill-opacity=".35" stroke="none"/>',
  zonalStats:
    '<path d="M3.2 6.2 9.5 2.5l7.6 3.3-.8 9.6-8.8 2.4-4.5-4.4z" fill="currentColor" fill-opacity=".12"/><path d="M6.8 14.5v-3.8M9.6 14.5V8M12.4 14.5V9.6" stroke-width="2"/>',
  histogram:
    '<path d="M1.8 17.5h16.4" stroke-width="1.2"/><rect x="2.8" y="12" width="2.5" height="5.5" fill="currentColor" fill-opacity=".35"/><rect x="5.9" y="7" width="2.5" height="10.5" fill="currentColor" fill-opacity=".6"/><rect x="9" y="3" width="2.5" height="14.5" fill="currentColor" fill-opacity=".85"/><rect x="12.1" y="8.5" width="2.5" height="9" fill="currentColor" fill-opacity=".6"/><rect x="15.2" y="13" width="2.5" height="4.5" fill="currentColor" fill-opacity=".35"/>',
  focalStats:
    '<path d="M2 5.5h16M2 9h16M2 12.5h16M2 16h16M5.5 2v16M9 2v16M12.5 2v16M16 2v16" stroke-width=".6" stroke-opacity=".4"/><rect x="5.5" y="5.5" width="10.5" height="10.5" fill="currentColor" fill-opacity=".14" stroke-width="1.6"/><rect x="9" y="9" width="3.5" height="3.5" fill="currentColor" stroke="none"/>',
  cellStats:
    '<path d="M2 13.5 8 16.8l10-4.3-6-3.3z" fill="currentColor" fill-opacity=".12"/><path d="M2 9.5 8 12.8l10-4.3-6-3.3z" fill="currentColor" fill-opacity=".12"/><path d="M2 5.5 8 8.8l10-4.3-6-3.3z" fill="currentColor" fill-opacity=".12"/><path d="M10.2 3.8v11" stroke-width="1.6"/><circle cx="10.2" cy="3.8" r="1.2" fill="currentColor" stroke="none"/><circle cx="10.2" cy="7.8" r="1.2" fill="currentColor" stroke="none"/><circle cx="10.2" cy="11.8" r="1.2" fill="currentColor" stroke="none"/>',
  rasterize:
    '<path d="M2 6h16M2 10h16M2 14h16M6 2v16M10 2v16M14 2v16" stroke-width=".8" stroke-opacity=".35"/><rect x="2" y="2" width="16" height="16" rx="1" stroke-width="1" stroke-opacity=".35"/><path d="M6 6h4v4h4v4H6z" fill="currentColor" fill-opacity=".5" stroke="none"/><path d="M4.4 4.4 15.6 9 8.6 16.4z" stroke-width="1.5"/>',
  toPolygons:
    '<path d="M2 6h16M2 10h16M2 14h16M6 2v16M10 2v16M14 2v16" stroke-width=".8" stroke-opacity=".35"/><rect x="2" y="2" width="16" height="16" rx="1" stroke-width="1" stroke-opacity=".35"/><path d="M2 2h8v4h4v8H6v4H2z" fill="currentColor" fill-opacity=".28" stroke-width="1.6"/><rect x="14" y="14" width="4" height="4" fill="currentColor" fill-opacity=".6" stroke="none"/>',
  toLines:
    '<path d="M2 12.5h3.5V16H2zM5.5 9.5H9V13H5.5zM9 9.5h3.5V13H9zM12.5 6h3.5v3.5h-3.5zM14.5 2.5H18V6h-3.5z" fill="currentColor" fill-opacity=".25" stroke="none"/><path d="M3.7 14.2 7.2 11.2h3.6l3.4-3.4 2-3.5" stroke-width="1.5"/>',
  toPoints:
    '<path d="M2 6h16M2 10h16M2 14h16M6 2v16M10 2v16M14 2v16" stroke-width=".8" stroke-opacity=".35"/><rect x="2" y="2" width="16" height="16" rx="1" stroke-width="1" stroke-opacity=".35"/><circle cx="4" cy="4" r="1.3" fill="currentColor" stroke="none"/><circle cx="12" cy="4" r="1.3" fill="currentColor" stroke="none"/><circle cx="4" cy="12" r="1.3" fill="currentColor" stroke="none"/><circle cx="8" cy="8" r="1.3" fill="currentColor" stroke="none"/><path d="M16 13.6 18.2 17.6H13.8z" fill="currentColor" stroke="none"/>',
  captureLine:
    '<path d="M2 12.5c3-1 5-6 9-6s4.5 3 7 1.5" stroke-width="4.2" stroke-opacity=".25"/><path d="M2 12.5c3-1 5-6 9-6s4.5 3 7 1.5" stroke-width="1.3"/><path d="M10.2 10.6v6.2l1.6-1.5 1.1 2.5 1.2-.5-1.1-2.5h2.2z" fill="currentColor" stroke="none"/>',
  closeArea:
    '<path d="M3 5.5 9 2.5l8 3.5-1.5 9.5-9.5 2z" stroke-width="3.6" stroke-opacity=".25"/><path d="M3 5.5 9 2.5l8 3.5-1.5 9.5-9.5 2z" fill="currentColor" fill-opacity=".18" stroke-width="1.3"/><path d="M8.6 7.4v6.2l1.6-1.5 1.1 2.5 1.2-.5-1.1-2.5h2.2z" fill="currentColor" stroke="none"/>',
  contourElevations:
    '<path d="M2 5.2c3-2 6 1 9-.5s5-2 7-.5M2 10c3-2 6 1 9-.5s5-2 7-.5M2 14.8c3-2 6 1 9-.5s5-2 7-.5" stroke-width="1.2"/><path d="M6.4 18.2 13.6 1.8" stroke-width="1.2" stroke-dasharray="1.6 1.3"/><circle cx="12.3" cy="4.8" r="1.3" fill="currentColor" stroke="none"/><circle cx="10.2" cy="9.5" r="1.3" fill="currentColor" stroke="none"/><circle cx="8.1" cy="14.3" r="1.3" fill="currentColor" stroke="none"/>',
  fillSinks:
    '<path d="M2 5.5c2.4 0 3 9 8 9s5.6-9 8-9" stroke-width="1.4"/><path d="M4.6 9.6c1 2.9 2.6 4.9 5.4 4.9s4.4-2 5.4-4.9z" fill="currentColor" fill-opacity=".34" stroke="none"/><path d="M3.8 9.6h12.4" stroke-width="1.1"/><path d="M10 2v4.4M8.2 4.8 10 6.6l1.8-1.8" stroke-width="1.3"/>',
  flowDirection:
    '<path d="M2 7.3h16M2 12.7h16M7.3 2v16M12.7 2v16" stroke-width=".7" stroke-opacity=".35"/><path d="M4.4 4.4 14.8 14.8" stroke-width="1.7"/><path d="M8.4 15.2h6.8V8.4" stroke-width="1.7"/>',
  flowAccumulation:
    '<path d="M2 7.3h16M2 12.7h16M7.3 2v16M12.7 2v16" stroke-width=".7" stroke-opacity=".35"/><rect x="2.6" y="2.6" width="4.1" height="4.1" fill="currentColor" fill-opacity=".2" stroke="none"/><rect x="13.3" y="2.6" width="4.1" height="4.1" fill="currentColor" fill-opacity=".2" stroke="none"/><rect x="7.9" y="7.9" width="4.2" height="4.2" fill="currentColor" fill-opacity=".55" stroke="none"/><rect x="7.9" y="13.3" width="4.2" height="4.1" fill="currentColor" stroke="none"/>',
  wetness:
    '<path d="M2 17c3-.8 4.5-3 8-3s5 2.2 8 3" stroke-width="1.2" stroke-opacity=".6"/><path d="M10 2.4c2.9 3.7 4.3 6.1 4.3 7.9a4.3 4.3 0 0 1-8.6 0c0-1.8 1.4-4.2 4.3-7.9z" fill="currentColor" fill-opacity=".28" stroke-width="1.4"/><path d="M8.2 10.6a1.9 1.9 0 0 0 1.6 1.8" stroke-width="1.2"/>',
  pourPoint:
    '<path d="M2.5 3c2.5 1 4 3 5 5.5S10 14 13.5 15" stroke-width="1.5"/><circle cx="14.6" cy="15.2" r="3.1" stroke-width="1.3"/><circle cx="14.6" cy="15.2" r="1.1" fill="currentColor" stroke="none"/><path d="M14.6 10.6v1.4M14.6 18.4v1.2M10 15.2h1.4M17.8 15.2h1.2" stroke-width="1.1"/>',
  watershed:
    '<path d="M4 4.5 9.5 2l7 2.8.8 6.3-5.3 6.6H8.4L2.6 11z" fill="currentColor" fill-opacity=".16" stroke-width="1.3"/><path d="M5.5 6.5c1.5 1 2.6 2.5 4.6 3.4M14.5 5.5 12 9.8M10.1 9.9l.2 7.8" stroke-width="1.2"/><circle cx="10.3" cy="17.6" r="1.5" fill="currentColor" stroke="none"/>',
  basins:
    '<path d="M3 4.5 9 2.2l.8 8.2-7.2 2.2z" fill="currentColor" fill-opacity=".32" stroke="none"/><path d="M9.8 10.4 18 10.2l-3 6.8-7 1.2-5.4-5.6z" fill="currentColor" fill-opacity=".16" stroke="none"/><path d="M3 4.5 9 2.2l7.2 1.6L18 10.2l-3 6.8-7 1.2-5.4-5.6z" stroke-width="1.3"/><path d="M9 2.2l.8 8.2M2.6 12.6l7.2-2.2L18 10.2" stroke-width="1.2"/>',
  streams:
    '<path d="M2.5 2.5 5 6.6M8 2 6.6 6.4" stroke-width="1"/><path d="M5 6.6l1.6-.2 2.6 5.3M17.5 5.5l-2.7 3.4M13.5 2.8l.9 4.8" stroke-width="1.5"/><path d="M14.4 7.6l.4 1.3-5.6 2.8M9.2 11.7l1.4 6.3" stroke-width="2.4"/>',
  distanceSurface:
    '<rect x="2.4" y="2.4" width="15.2" height="15.2" rx="1.4" stroke-width="1" stroke-opacity=".45"/><circle cx="10" cy="10" r="1.7" fill="currentColor" stroke="none"/><circle cx="10" cy="10" r="4.1" stroke-width="1.4"/><circle cx="10" cy="10" r="6.6" stroke-width="1.1" stroke-opacity=".6" stroke-dasharray="2.2 1.5"/>',
  costDistance:
    '<path d="M2 7.3h16M2 12.7h16M7.3 2v16M12.7 2v16" stroke-width=".7" stroke-opacity=".35"/><rect x="7.9" y="2.6" width="4.2" height="4.1" fill="currentColor" fill-opacity=".2" stroke="none"/><rect x="2.6" y="7.9" width="4.1" height="4.2" fill="currentColor" fill-opacity=".2" stroke="none"/><rect x="7.9" y="7.9" width="4.2" height="4.2" fill="currentColor" fill-opacity=".38" stroke="none"/><rect x="13.3" y="2.6" width="4.1" height="4.1" fill="currentColor" fill-opacity=".5" stroke="none"/><rect x="2.6" y="13.3" width="4.1" height="4.1" fill="currentColor" fill-opacity=".5" stroke="none"/><rect x="13.3" y="7.9" width="4.1" height="4.2" fill="currentColor" fill-opacity=".66" stroke="none"/><rect x="7.9" y="13.3" width="4.2" height="4.1" fill="currentColor" fill-opacity=".66" stroke="none"/><rect x="13.3" y="13.3" width="4.1" height="4.1" fill="currentColor" fill-opacity=".9" stroke="none"/><circle cx="4.65" cy="4.65" r="1.5" fill="currentColor" stroke="none"/>',
  costPath:
    '<path d="M6.6 5.4c2.6-.6 5 .9 5.2 3.3.2 2.2-1.6 3.8-4 3.9-2.5.1-4-1.4-4-3.4 0-1.8 1-3.4 2.8-3.8z" fill="currentColor" fill-opacity=".2" stroke="none"/><path d="M3.6 16.4 9.4 15.8 13.6 13 15 8.2 16.4 3.6" stroke-width="1.6"/><circle cx="3.6" cy="16.4" r="1.6" fill="currentColor" stroke="none"/><circle cx="16.4" cy="3.6" r="1.7" stroke-width="1.3"/>',
  costCorridor:
    '<path d="M4.2 15.8C4.2 8.6 15.8 11.4 15.8 4.2" stroke-width="6.2" stroke-opacity=".24"/><path d="M4.2 15.8C4.2 8.6 15.8 11.4 15.8 4.2" stroke-width="1.2"/><circle cx="4.2" cy="16.6" r="1.8" fill="currentColor" stroke="none"/><circle cx="15.8" cy="3.4" r="1.8" fill="currentColor" stroke="none"/>',
  server: '<rect x="3" y="3" width="14" height="5.5" rx="1"/><rect x="3" y="11.5" width="14" height="5.5" rx="1"/><path d="M6 5.75h.01M6 14.25h.01" stroke-width="2"/><path d="M9.5 5.75h4.5M9.5 14.25h4.5"/>',
  cloud: '<path d="M6 15.5a3.5 3.5 0 0 1-.4-7A4.8 4.8 0 0 1 14.8 7a3.3 3.3 0 0 1-.3 8.5z"/><path d="M10 9v4.6M7.9 11.6 10 13.7l2.1-2.1"/>',
  cloudUpload: '<path d="M6 15.5a3.5 3.5 0 0 1-.4-7A4.8 4.8 0 0 1 14.8 7a3.3 3.3 0 0 1-.3 8.5z"/><path d="M10 13.8V9.2M7.9 11.1 10 9l2.1 2.1"/>',
  conflict: '<path d="M4 3.5v5.2a3 3 0 0 0 3 3h6a3 3 0 0 1 3 3v2"/><path d="M16 3.5v5.2a3 3 0 0 1-3 3H7a3 3 0 0 0-3 3v2"/><circle cx="4" cy="3.5" r="1.4" fill="currentColor" stroke="none"/><circle cx="16" cy="3.5" r="1.4" fill="currentColor" stroke="none"/>',
  signIn: '<path d="M11 3.5h4.5v13H11"/><path d="M3 10h9M9 6.8l3.2 3.2L9 13.2"/>',
  signOut: '<path d="M9 3.5H4.5v13H9"/><path d="M8 10h9M14 6.8l3.2 3.2-3.2 3.2"/>',
  // Sharing a project: two people.
  share: '<circle cx="7.5" cy="7" r="2.6"/><path d="M2.8 16.3c.4-2.9 2.3-4.5 4.7-4.5s4.3 1.6 4.7 4.5"/><path d="M12.6 4.6a2.5 2.5 0 0 1 0 4.8M14 11.9c1.8.4 3 1.9 3.3 4.4"/>',
  penTool: '<path d="m10 2.8 4.6 6.6-2.2 5.1H7.6L5.4 9.4z"/><path d="M10 2.8v6.1"/><circle cx="10" cy="10.2" r="1.3"/><path d="M7.6 17.2h4.8"/>',
  legend: '<rect x="3" y="3.5" width="4" height="3" rx=".5"/><rect x="3" y="8.5" width="4" height="3" rx=".5" fill="currentColor" fill-opacity=".3"/><path d="M3 15h4" stroke-dasharray="1.4 1.2"/><path d="M9.5 5h7.5M9.5 10h7.5M9.5 15h5"/>',
  // Nesne şablonları (docs/adr/0176): the panel, a stack of template cards with a parcel on the front one; Şablonla
  // çiz, a card with its parcel and the pencil that draws with it; Yeni şablon, a card and a plus; Seçili nesneden
  // şablon, a selected object (its grips) going into a card; Şablonu uygula, a card going onto a selected object.
  templates:
    '<path d="M6.5 5.2V3.7a1.2 1.2 0 0 1 1.2-1.2h8.6a1.2 1.2 0 0 1 1.2 1.2v8.1a1.2 1.2 0 0 1-1.2 1.2h-1.1"/><rect x="2.5" y="5.5" width="12.5" height="12" rx="1.2" fill="currentColor" fill-opacity=".1"/><path d="m5.3 14.6 1.4-5.1 4.8 1.2-.7 3.9z" fill="currentColor" fill-opacity=".35" stroke-width="1.1"/>',
  templateDraw:
    '<rect x="2.5" y="2.5" width="10" height="10" rx="1.2" fill="currentColor" fill-opacity=".1"/><path d="m4.9 10 1.1-4.2 4 1-.6 3.2z" fill="currentColor" fill-opacity=".35" stroke-width="1.1"/><path d="m11.2 17.6.7-2.8 5-5a1.35 1.35 0 0 1 1.9 1.9l-5 5z"/><path d="m15.6 11.1 1.9 1.9" stroke-width="1.1"/>',
  templateNew:
    '<rect x="2.5" y="2.5" width="11" height="11" rx="1.2" fill="currentColor" fill-opacity=".1"/><path d="m5 10.8 1.2-4.4 4.2 1-.6 3.4z" fill="currentColor" fill-opacity=".35" stroke-width="1.1"/><path d="M15.5 12v6M12.5 15h6"/>',
  templateFromSelection: `<path d="m3 8 1.4-4.7 4.8 1.3-.9 4.3z" fill="currentColor" fill-opacity=".25" stroke-width="1.1" stroke-dasharray="1.6 1.3"/>${grip(3, 8)}${grip(4.4, 3.3)}${grip(9.2, 4.6)}<rect x="9.5" y="9.5" width="8" height="8" rx="1.1" fill="currentColor" fill-opacity=".1"/><path d="m11.4 15.4.9-3.3 3.1.8-.5 2.5z" fill="currentColor" fill-opacity=".35" stroke-width="1"/><path d="M5.5 11.2v2.3a1.5 1.5 0 0 0 1.5 1.5h1.4m-1.5-1.6 1.6 1.6-1.6 1.6" stroke-width="1.2"/>`,
  templateApply: `<rect x="2.5" y="2.5" width="9" height="9" rx="1.2" fill="currentColor" fill-opacity=".1"/><path d="m4.6 9.4 1-3.7 3.6.9-.5 2.8z" fill="currentColor" fill-opacity=".35" stroke-width="1.1"/><path d="M13 5h1.5a1.5 1.5 0 0 1 1.5 1.5v2.8m-1.6-1.6 1.6 1.6 1.6-1.6" stroke-width="1.2"/><rect x="10.5" y="12" width="7" height="5.5" fill="currentColor" fill-opacity=".25" stroke-width="1.1" stroke-dasharray="1.6 1.3"/>${grip(10.5, 12)}${grip(17.5, 17.5)}`,
  symbolAssign: '<path d="m7.5 2.5 1.4 2.9 3.1.4-2.3 2.2.6 3.1-2.8-1.5-2.8 1.5.6-3.1-2.3-2.2 3.1-.4z"/><path d="M14.5 11v6.5M11.25 14.25h6.5"/>',
  symbolClear: '<path d="m7.5 2.5 1.4 2.9 3.1.4-2.3 2.2.6 3.1-2.8-1.5-2.8 1.5.6-3.1-2.3-2.2 3.1-.4z"/><path d="m12.2 12 4.6 4.6M16.8 12l-4.6 4.6"/>',
  // Symbols' kinds and their layers (Stil yöneticisi, Sembol tasarımcısı): a line with marks along it, a pin;
  // a solid fill, a pattern of dots, a picture filling the area, a stroke.
  symbolLine: '<path d="M2.5 14 17.5 6"/><path d="M4.7 10.9l1.6 3M9.2 8.5l1.6 3M13.7 6.1l1.6 3" stroke-width="1.1"/>',
  symbolMarker: '<path d="M10 17.5S4.5 12.1 4.5 8a5.5 5.5 0 0 1 11 0c0 4.1-5.5 9.5-5.5 9.5z"/><circle cx="10" cy="8" r="2"/>',
  fillSolid: '<rect x="3.5" y="3.5" width="13" height="13" rx="1" fill="currentColor" fill-opacity=".35"/>',
  fillPattern: `<rect x="3.5" y="3.5" width="13" height="13" rx="1"/>${[6.75, 10, 13.25].flatMap((y) => [6.75, 10, 13.25].map((x) => `<circle cx="${x}" cy="${y}" r=".9" fill="currentColor" stroke="none"/>`)).join('')}`,
  fillImage: '<rect x="3.5" y="3.5" width="13" height="13" rx="1"/><circle cx="7.5" cy="7.5" r="1.4"/><path d="m3.5 14 3.6-3.2 2.6 2 2.6-2.6 4.2 3.8"/>',
  symbolStroke: '<path d="M3 14.5c3.5-7 7.5-7 9-3.5s3.5 2.5 5-3" stroke-width="2"/>',
  // Ribbon chrome.
  ribbon: '<rect x="2.5" y="3" width="15" height="14" rx="1"/><path d="M2.5 6.2h15M2.5 11h15"/><rect x="4.5" y="7.4" width="2.4" height="2.4" rx=".4"/><path d="M8.8 7.9h2.6M8.8 9.4h2.6M13 7.9h2.5"/>',
  launcher: '<path d="M4.5 4.5h11v11"/><path d="m15.5 15.5-7-7M15.5 10.5v5h-5"/>',
  help: '<circle cx="10" cy="10" r="7"/><path d="M7.9 7.8a2.2 2.2 0 0 1 4.2.8c0 1.5-2.1 1.8-2.1 3.2M10 14.1v.1"/>',
  pin: '<path d="M12.5 3.5 16.5 7.5 13 9.5 10.5 12l-.6 3.4L4.6 10l3.4-.6L10.5 7z"/><path d="m7.3 12.7-4 4"/>',
  plus: '<path d="M10 4v12M4 10h12"/>',
  trash: '<path d="M4 5.5h12M8 5.5v-2h4v2M5.6 5.5l.8 11h7.2l.8-11"/><path d="M8.6 8.5v5M11.4 8.5v5"/>',
  // The project catalog (docs/adr/0028): a favourite (outline; marked, a deliberate 35 % fill) and an archive box.
  star: '<path d="m10 3.2 2.1 4.3 4.7.7-3.4 3.3.8 4.7-4.2-2.2-4.2 2.2.8-4.7-3.4-3.3 4.7-.7z"/>',
  starOn: '<path d="m10 3.2 2.1 4.3 4.7.7-3.4 3.3.8 4.7-4.2-2.2-4.2 2.2.8-4.7-3.4-3.3 4.7-.7z" fill="currentColor" fill-opacity=".35"/>',
  archive: '<path d="M3 4.5h14v3H3z"/><path d="M4.5 7.5v8h11v-8M8.2 10.5h3.6"/>',
  // Yazı's Hiza and Öznitelikler (docs/adr/0145 §6): `textAlign` + the alignment's name, the left of the baseline too.
  textAlignTopLeft: textAlign(3, 4.5),
  textAlignTopCenter: textAlign(10, 4.5),
  textAlignTopRight: textAlign(17, 4.5),
  textAlignMiddleLeft: textAlign(3, 8.5),
  textAlignMiddleCenter: textAlign(10, 8.5),
  textAlignMiddleRight: textAlign(17, 8.5),
  textAlignBottomLeft: textAlign(3, 15.5),
  textAlignBottomCenter: textAlign(10, 15.5),
  textAlignBottomRight: textAlign(17, 15.5),
  textAlignBaselineLeft: textAlign(3, 12.5),
  textAlignBaselineCenter: textAlign(10, 12.5),
  textAlignBaselineRight: textAlign(17, 12.5),
  // The hatch patterns, each drawn by itself (docs/adr/0186 §2; written by scripts/ui/hatch_icons.py), and their
  // wide samples for Desen's menus (`iconPreview`; the desktop draws them from the inventory).
  ...HATCH_ICONS,
  ...HATCH_PREVIEWS,
  // The leader arrowheads, each drawn from its shape (docs/adr/0205 §7; written by scripts/ui/arrow_icons.py).
  ...ARROW_ICONS,
  // Map services and the ready basemaps (docs/adr/0208 §14).
  ...SERVICE_ICONS,
  // Zaman and Senaryo (docs/adr/0210 §10): a slider under a clock; a layer with a clock; a version's old shape dashed beside
  // its new one; an hourglass; two clocks; a branch with a plus, an eye, its trunk ticked, beside ≠ and merged back.
  timeSlider: '<circle cx="10" cy="7" r="4.2"/><path d="M10 4.9V7l1.5 1.1"/><path d="M2.5 15h15"/><rect x="11.4" y="13" width="2.6" height="4" rx=".8" fill="currentColor" stroke="none"/>',
  timeLayer: '<path d="M2.5 8.2 8.5 5l6 3.2-6 3.2z"/><path d="m2.5 11.4 6 3.2 2.2-1.2"/><circle cx="14.6" cy="13.8" r="3.4"/><path d="M14.6 12.1v1.8l1.2.8"/>',
  timeVersion: '<rect x="2.5" y="5" width="6" height="10" rx="1" stroke-dasharray="2 1.6"/><rect x="11.5" y="5" width="6" height="10" rx="1"/><path d="M8.5 10h3M10.2 8.5l1.3 1.5-1.3 1.5"/>',
  timeEnd: '<path d="M5.5 3h9M5.5 17h9"/><path d="M6.5 3c0 3.6 3.5 4.6 3.5 7s-3.5 3.4-3.5 7M13.5 3c0 3.6-3.5 4.6-3.5 7s3.5 3.4 3.5 7"/><path d="M8 15.5h4" stroke-width="2"/>',
  timeCompare: '<circle cx="6.2" cy="10" r="4"/><circle cx="13.8" cy="10" r="4"/><path d="M6.2 8v2H8M13.8 8v2l1.4 1"/>',
  scenarioCreate: '<circle cx="5" cy="15.5" r="1.8"/><circle cx="5" cy="4.5" r="1.8"/><path d="M5 6.3v7.4M5 11.5c0-2.6 1.8-4.2 4.6-4.2"/><path d="M14.5 3.5v7M11 7h7"/>',
  scenarioShow: '<circle cx="5" cy="15.5" r="1.8"/><path d="M5 13.7V3.5M5 9.5c0-2.6 1.8-4.2 4.6-4.2h1"/><path d="M9.5 14c1.7-2.4 6.3-2.4 8 0-1.7 2.4-6.3 2.4-8 0z"/><circle cx="13.5" cy="14" r="1.1" fill="currentColor" stroke="none"/>',
  scenarioBase: '<circle cx="5" cy="15.5" r="1.8"/><circle cx="5" cy="4.5" r="1.8"/><path d="M5 6.3v7.4" stroke-width="2"/><path d="M5 11.5c0-2.6 1.8-4.2 4.6-4.2h1.6" stroke-dasharray="1.6 1.6"/><path d="m11.5 13.5 2 2 4-4.5"/>',
  scenarioCompare: '<circle cx="5" cy="15.5" r="1.8"/><path d="M5 13.7V3.5M5 9.5c0-2.6 1.8-4.2 4.6-4.2"/><path d="M11 10.5h6.5M11 14h6.5M15.6 8.2l-2.7 8"/>',
  scenarioApply: '<circle cx="5" cy="16" r="1.8"/><path d="M5 14.2V3.5"/><path d="M15.5 3.5c0 4.4-3.2 6.6-8.8 7.4"/><path d="M8.9 8.8 6.5 11l2.6 1.8"/>',
  // A scenario group in the layer tree: a branch off the trunk; a temporal layer's badge: a clock.
  scenario: '<circle cx="5" cy="15.5" r="1.8"/><circle cx="5" cy="4.5" r="1.8"/><circle cx="14.5" cy="6.5" r="1.8"/><path d="M5 6.3v7.4M5 12c0-3.2 2.6-4.8 7.8-5.4"/>',
  clock: '<circle cx="10" cy="10" r="6.5"/><path d="M10 6.2V10l2.6 1.7"/>',
  timeFirst: '<path d="M5 5v10M14.5 5l-5 5 5 5"/>',
  timePrev: '<path d="M12.5 5l-5 5 5 5"/>',
  timeNext: '<path d="M7.5 5l5 5-5 5"/>',
  timeLast: '<path d="M15 5v10M5.5 5l5 5-5 5"/>',
  pause: '<rect x="5.5" y="4.5" width="3" height="11" rx=".8" fill="currentColor" stroke="none"/><rect x="11.5" y="4.5" width="3" height="11" rx=".8" fill="currentColor" stroke="none"/>',
  loop: '<path d="M3.5 10.5V9a4 4 0 0 1 4-4h7.5"/><path d="M13 2.8 15.2 5 13 7.2"/><path d="M16.5 9.5V11a4 4 0 0 1-4 4H5"/><path d="M7 17.2 4.8 15 7 12.8"/>',
  // Katman süzgeci (docs/adr/0211 §4): a layer with a funnel; a dashed selection box round two grips with a funnel; a
  // funnel and a cross. A filtered layer's badge in the tree: a funnel, filled to read at 12 px.
  layerFilter: '<path d="M2.5 8.2 8.5 5l6 3.2-6 3.2z"/><path d="m2.5 11.4 6 3.2 2.2-1.2"/><path d="M11 10.5h7.5l-2.9 3.4v3.7l-1.7-.9v-2.8z"/>',
  layerFilterSelection: `<rect x="2.5" y="3" width="10" height="9.5" rx="1" stroke-dasharray="2 1.6"/>${grip(5.5, 6.5)}${grip(9.2, 9.5)}<path d="M11 10.5h7.5l-2.9 3.4v3.7l-1.7-.9v-2.8z"/>`,
  layerFilterClear: '<path d="M2.5 3.5h12l-4.6 5.4v6.2l-2.8-1.4V8.9z"/><path d="m13 12 4.5 4.5m0-4.5L13 16.5"/>',
  funnel: '<path d="M3 4h14l-5.5 6.5V16l-3-1.5v-4z" fill="currentColor" fill-opacity=".3"/>',
  // Etiket motoru (docs/adr/0212 §4): a label's tag (its hole a dot) with what is done to it.
  layerLabels:
    '<path d="M2.5 8.2 8.5 5l6 3.2-6 3.2z"/><path d="m2.5 11.4 6 3.2 1.4-.75"/><path d="M11 11.5h4.4l2.6 2.5-2.6 2.5H11z"/><circle cx="12.9" cy="14" r=".85" fill="currentColor" stroke="none"/>',
  labelMove:
    '<path d="M2.5 3.5h6.6l2.5 2.6-2.5 2.6H2.5z"/><circle cx="4.6" cy="6.1" r=".85" fill="currentColor" stroke="none"/><path d="M14 10.5v7M10.5 14h7"/><path d="M12.8 11.7 14 10.5l1.2 1.2M12.8 16.3l1.2 1.2 1.2-1.2M11.7 12.8 10.5 14l1.2 1.2M16.3 12.8l1.2 1.2-1.2 1.2"/>',
  labelRotate:
    '<path d="M2.5 3.5h6.6l2.5 2.6-2.5 2.6H2.5z"/><circle cx="4.6" cy="6.1" r=".85" fill="currentColor" stroke="none"/><path d="M17.5 14a3.9 3.9 0 1 1-1.3-2.9"/><path d="M16.6 8.6v2.6H14"/>',
  labelPin:
    '<path d="M2.5 11.5h6.2l2.4 2.5-2.4 2.5H2.5z"/><circle cx="4.5" cy="14" r=".85" fill="currentColor" stroke="none"/><path d="m13.6 2.5 3.9 3.9-2 1.3-1.5 1.5-.4 2.2-5-5 2.2-.4 1.5-1.5z"/><path d="m10.5 8.6-2.3 2.3"/>',
  labelHide:
    '<path d="M3 6.6h9.4l3 3.2-3 3.2H3z"/><circle cx="5.4" cy="9.8" r=".85" fill="currentColor" stroke="none"/><path d="m3.2 3.2 13.6 13.6"/>',
  labelsPinned:
    '<rect x="2" y="5" width="14.5" height="10" rx="1" stroke-dasharray="2.2 1.6"/><path d="M4.5 7.9h6.3l2.2 2.1-2.2 2.1H4.5z"/><circle cx="16.6" cy="4.9" r="1.7" fill="currentColor" stroke="none"/>',
  labelsUnplaced:
    '<path d="M2.5 4h6.6l2.5 2.6-2.5 2.6H2.5z"/><path d="M7.5 10.8h6.6l2.5 2.6-2.5 2.6H7.5z" stroke-dasharray="2 1.5"/><circle cx="4.6" cy="6.6" r=".85" fill="currentColor" stroke="none"/>',
  // Katman stili's renderers (docs/adr/0213 §4): one glyph each, the list and the legend's window alike.
  rendererSimple: '<rect x="3.5" y="3.5" width="13" height="13" rx="1.5"/><path d="M3.5 16.5 16.5 3.5"/>',
  rendererSingle: '<circle cx="10" cy="10" r="5.5" fill="currentColor" fill-opacity=".3"/>',
  rendererCategorized: '<circle cx="4.8" cy="10" r="2.3"/><rect x="8" y="7.7" width="4.6" height="4.6"/><path d="m15.4 7.4 2.7 4.8h-5.4z"/>',
  rendererGraduated: '<path d="M3.5 16.5h3v-4h-3zM8.5 16.5h3v-8h-3zM13.5 16.5h3v-12h-3z"/>',
  rendererUnclassed:
    '<rect x="2.5" y="6.5" width="15" height="7" rx="1"/><path d="M6.25 6.5v7M10 6.5v7M13.75 6.5v7"/><path d="M6.25 7.2h3.75v5.6H6.25z" fill="currentColor" fill-opacity=".25" stroke="none"/><path d="M10 7.2h3.75v5.6H10z" fill="currentColor" fill-opacity=".55" stroke="none"/><path d="M13.75 7.2h3.05v5.6h-3.05z" fill="currentColor" fill-opacity=".9" stroke="none"/>',
  rendererProportional: '<circle cx="4.3" cy="14.2" r="1.6"/><circle cx="8.9" cy="12.8" r="3"/><circle cx="14.6" cy="11" r="4.6"/>',
  rendererBivariate:
    '<rect x="3" y="3" width="14" height="14" rx="1"/><path d="M10 3v14M3 10h14"/><path d="M3.6 3.6H9.4V9.4H3.6z" fill="currentColor" fill-opacity=".45" stroke="none"/><path d="M10.6 3.6h5.8v5.8h-5.8z" fill="currentColor" fill-opacity=".9" stroke="none"/><path d="M10.6 10.6h5.8v5.8h-5.8z" fill="currentColor" fill-opacity=".45" stroke="none"/>',
  rendererRules: '<path d="M3 5h14M5.5 10h9M8 15h4"/>',
  rendererDotDensity:
    '<path d="M3 15.8 4.6 4.4l12.2 1.8.7 9.8z"/><g fill="currentColor" stroke="none"><circle cx="7" cy="7.8" r=".9"/><circle cx="11.5" cy="8.6" r=".9"/><circle cx="8.8" cy="11.6" r=".9"/><circle cx="13.6" cy="12.6" r=".9"/><circle cx="6.4" cy="13.2" r=".9"/></g>',
  rendererChart: '<circle cx="10" cy="10" r="6.5"/><path d="M10 3.5V10l5.6 3.3"/><path d="M10 3.5V10l5.6 3.3A6.5 6.5 0 0 0 10 3.5z" fill="currentColor" fill-opacity=".35" stroke="none"/>',
  rendererHeatmap:
    '<circle cx="10" cy="10" r="7" fill="currentColor" fill-opacity=".12" stroke="none"/><circle cx="10" cy="10" r="4.4" fill="currentColor" fill-opacity=".3" stroke="none"/><circle cx="10" cy="10" r="1.9" fill="currentColor" stroke="none"/><circle cx="10" cy="10" r="7"/>',
  rendererCluster:
    '<circle cx="10" cy="10" r="6.5"/><g fill="currentColor" stroke="none"><circle cx="8" cy="8.4" r="1.1"/><circle cx="12.2" cy="9.2" r="1.1"/><circle cx="9.6" cy="12.3" r="1.1"/></g><circle cx="3" cy="16.5" r="1" fill="currentColor" stroke="none"/>',
  rendererDisplacement:
    '<circle cx="10" cy="10" r="6" stroke-dasharray="1.6 1.9"/><circle cx="10" cy="10" r="1.2" fill="currentColor" stroke="none"/><g fill="currentColor" stroke="none"><circle cx="10" cy="4" r="1.4"/><circle cx="15.7" cy="11.9" r="1.4"/><circle cx="4.3" cy="11.9" r="1.4"/></g>',
  rendererInverted:
    '<path d="M2.5 2.5h15v15h-15zM7 6.5l6.5 1.2-1.2 6-5.8-1.4z" fill="currentColor" fill-opacity=".3" fill-rule="evenodd" stroke="none"/><path d="M2.5 2.5h15v15h-15z"/><path d="M7 6.5l6.5 1.2-1.2 6-5.8-1.4z"/>',
} as const;

export type IconName = keyof typeof ICONS;

const NS = 'http://www.w3.org/2000/svg';

export function icon(name: string, size = 18): SVGSVGElement {
  const svg = document.createElementNS(NS, 'svg');
  svg.setAttribute('viewBox', '0 0 20 20');
  svg.setAttribute('width', String(size));
  svg.setAttribute('height', String(size));
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '1.4');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.setAttribute('aria-hidden', 'true');
  svg.classList.add('icon');
  svg.innerHTML = (ICONS as Record<string, string>)[name] ?? ICONS.more;
  return svg;
}

/**
 * A wide sample (Desen's menus, docs/adr/0186 §11): 56 × 24 px of a drawing in a 46.67 × 20 box of the icon grid's
 * units, so that its strokes are an icon's.
 */
export function iconPreview(name: string): SVGSVGElement {
  const svg = icon(name);
  svg.setAttribute('viewBox', '0 0 46.667 20');
  svg.setAttribute('width', '56');
  svg.setAttribute('height', '24');
  svg.classList.add('icon--preview');
  return svg;
}
