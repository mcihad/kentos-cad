import { icon } from '../icons';

/**
 * Icons of the SVG editor's path, node and arrange commands, drawn like
 * the app's set (ui/icons.ts: 20×20, 1.4 px stroke, filled squares are
 * nodes, a light fill is the result). Names the app already has (fillet,
 * chamfer, measure, array, mirror, lock …) come from there.
 */

const node = (x: number, y: number) => `<rect x="${x - 1.6}" y="${y - 1.6}" width="3.2" height="3.2" fill="currentColor" stroke="none"/>`;
const fillA = 'fill="currentColor" fill-opacity=".28"';
const A = 'M3 3h9v9H3z';
const B = 'M8 8h9v9H8z';

const ICONS: Record<string, string> = {
  // Path operations: two squares, bottom A and top B.
  pathUnion: `<path d="M3 3h9v5h5v9H8v-5H3z" ${fillA}/>`,
  pathDifference: `<path d="M3 3h9v5H8v4H3z" ${fillA}/><path d="${B}" stroke-dasharray="2 1.6" opacity=".6"/>`,
  pathIntersection: `<path d="${A}M${B.slice(1)}" stroke-dasharray="2 1.6" opacity=".6"/><path d="M8 8h4v4H8z" ${fillA}/>`,
  pathExclusion: `<path d="${A}${B}" fill-rule="evenodd" ${fillA}/>`,
  pathDivision: `<path d="${A}"/><path d="M8 8h4v4H8z" ${fillA}/><path d="M12 8h5v9H8v-5" stroke-dasharray="2 1.6" opacity=".6"/>`,
  pathCut: `<path d="M8 3H3v9h5M12 8V3H8"/><path d="${B}" stroke-dasharray="2 1.6" opacity=".6"/>${node(8, 3)}${node(12, 8)}`,
  pathCombine: `<path d="M3 4h6v6H3zM11 10h6v6h-6z" ${fillA}/><path d="M9 7h5v3" stroke-dasharray="1.6 1.4"/>`,
  pathBreak: `<path d="M2.5 3.5h6v6h-6zM11.5 10.5h6v6h-6z" ${fillA}/><path d="m9.5 12-2 2m0-2 2 2" />`,
  pathSplit: `<path d="M3 3h9v9H3zM5.5 5.5v4h4v-4z" fill-rule="evenodd" ${fillA}/><path d="M14 11h4v6h-4z"/>`,
  toPath: `<rect x="4" y="5" width="12" height="10" rx="1"/>${node(4, 5)}${node(16, 5)}${node(16, 15)}${node(4, 15)}`,
  strokeToPath: `<path d="M4.5 7.5h11a2.5 2.5 0 0 1 0 5h-11a2.5 2.5 0 0 1 0-5z" ${fillA}/>`,
  inset: `<path d="M3 3h14v14H3z"/><path d="M7 7h6v6H7z" ${fillA}/><path d="m4.5 4.5 1.8 1.8m9.2-1.8-1.8 1.8" />`,
  outset: `<path d="M6 6h8v8H6z" ${fillA}/><path d="M2.5 2.5h15v15h-15z" stroke-dasharray="2 1.6"/>`,
  simplify: `<path d="M3 14 5 9l2 3 2-6 2 5 2-4 2 3 2-4" opacity=".45"/><path d="M3 15c3-8 11-8 14-6"/>`,
  reverse: `<path d="M4 13c2-7 10-7 12 0"/><path d="m13.5 12 2.5 1.2.6-2.7M6.5 7.5 4 6.5l-.3 2.6"/>`,
  closePath: `<path d="M4 15V5h12v10"/><path d="M4 15h12" stroke-dasharray="2 1.6"/>${node(4, 15)}${node(16, 15)}`,
  openPath: `<path d="M9 15H4V5h12v10h-3"/>${node(9, 15)}${node(13, 15)}`,
  // Node types: the node and its handles.
  nodeCusp: `<path d="M4 5 10 11l6-6"/><circle cx="4" cy="5" r="1.4"/><circle cx="16" cy="5" r="1.4"/><path d="M10 8.5 12.5 11 10 13.5 7.5 11z" fill="currentColor" stroke="none"/>`,
  nodeSmooth: `<path d="M3 10h14"/><circle cx="3" cy="10" r="1.4"/><circle cx="17" cy="10" r="1.4"/><rect x="6.4" y="8.4" width="3.2" height="3.2" fill="currentColor" stroke="none"/><path d="M3 15c3-5 11-5 14-9" opacity=".45"/>`,
  nodeSymmetric: `<path d="M3.5 10h13"/><circle cx="3.5" cy="10" r="1.4"/><circle cx="16.5" cy="10" r="1.4"/><rect x="8.4" y="8.4" width="3.2" height="3.2" fill="currentColor" stroke="none"/><path d="M3 15c3-5 11-5 14 0" opacity=".45"/>`,
  nodeAuto: `<path d="M3 15c3-9 11-9 14 0"/><circle cx="10" cy="8.2" r="2" fill="currentColor" stroke="none"/>`,
  // Node operations.
  nodeInsert: `<path d="M3 13c4-6 10-6 14 0"/>${node(3, 13)}${node(17, 13)}<path d="M10 3.5v5M7.5 6h5"/>`,
  nodeDelete: `<path d="M3 13c4-6 10-6 14 0"/>${node(3, 13)}${node(17, 13)}<path d="m8 4.5 4 4m0-4-4 4"/>`,
  nodeJoin: `<path d="M3 15 9 9m8 6-6-6"/>${node(10, 8.5)}<path d="M10 3v2.5" opacity=".5"/>`,
  nodeJoinSeg: `<path d="M3 15 7 9m10 6-4-6"/><path d="M7 9h6" stroke-dasharray="1.6 1.4"/>${node(7, 9)}${node(13, 9)}`,
  nodeBreak: `<path d="M3 14 8 9m9 5-5-5"/>${node(8, 9)}${node(12, 9)}`,
  segDelete: `<path d="M3 14l3-6m11 6-3-6"/><path d="M6 8h8" stroke-dasharray="1.6 1.4" opacity=".6"/>${node(6, 8)}${node(14, 8)}<path d="m8.5 11 3 3m0-3-3 3"/>`,
  segLine: `<path d="M4 14 16 6"/>${node(4, 14)}${node(16, 6)}`,
  segCurve: `<path d="M4 14C5 6 12 12 16 6"/>${node(4, 14)}${node(16, 6)}`,
  // Align and distribute (Inkscape's: the line is the reference).
  alignLeft: '<path d="M3 3v14"/><path d="M5 5h10v3H5zM5 11h6v3H5z" fill="currentColor" fill-opacity=".28"/>',
  alignHCenter: '<path d="M10 2.5v15"/><path d="M4 5h12v3H4zM7 11h6v3H7z" fill="currentColor" fill-opacity=".28"/>',
  alignRight: '<path d="M17 3v14"/><path d="M5 5h10v3H5zM9 11h6v3H9z" fill="currentColor" fill-opacity=".28"/>',
  alignTop: '<path d="M3 3h14"/><path d="M5 5h3v10H5zM11 5h3v6h-3z" fill="currentColor" fill-opacity=".28"/>',
  alignVCenter: '<path d="M2.5 10h15"/><path d="M5 4h3v12H5zM11 7h3v6h-3z" fill="currentColor" fill-opacity=".28"/>',
  alignBottom: '<path d="M3 17h14"/><path d="M5 5h3v10H5zM11 9h3v6h-3z" fill="currentColor" fill-opacity=".28"/>',
  distLeft: '<path d="M3 3v14M9 3v14M15 3v14" opacity=".5"/><path d="M3 6h4v8H3zM9 8h3v4H9zM15 5h2v10h-2z" fill="currentColor" fill-opacity=".28"/>',
  distHCenter: '<path d="M4 3v14M10 3v14M16 3v14" opacity=".5"/><path d="M2.5 6h3v8h-3zM8 8h4v4H8zM15 5h2v10h-2z" fill="currentColor" fill-opacity=".28"/>',
  distRight: '<path d="M7 3v14M13 3v14M17 3v14" opacity=".5"/><path d="M3 6h4v8H3zM10 8h3v4h-3zM15 5h2v10h-2z" fill="currentColor" fill-opacity=".28"/>',
  distHGap: '<path d="M2 6h3v8H2zM8.5 8h3v4h-3zM15 5h3v10h-3z" fill="currentColor" fill-opacity=".28"/><path d="M5.5 10h2.5m4 0h2.5"/>',
  distTop: '<path d="M3 3h14M3 9h14M3 15h14" opacity=".5"/><path d="M6 3h8v4H6zM8 9h4v3H8zM5 15h10v2H5z" fill="currentColor" fill-opacity=".28"/>',
  distVCenter: '<path d="M3 4h14M3 10h14M3 16h14" opacity=".5"/><path d="M6 2.5h8v3H6zM8 8h4v4H8zM5 15h10v2H5z" fill="currentColor" fill-opacity=".28"/>',
  distBottom: '<path d="M3 7h14M3 13h14M3 17h14" opacity=".5"/><path d="M6 3h8v4H6zM8 10h4v3H8zM5 15h10v2H5z" fill="currentColor" fill-opacity=".28"/>',
  distVGap: '<path d="M6 2h8v3H6zM8 8.5h4v3H8zM5 15h10v3H5z" fill="currentColor" fill-opacity=".28"/><path d="M10 5.5V8m0 4v2.5"/>',
  // Stacking order.
  toTop: '<path d="M4 3h12M10 16V6M6.5 9.5 10 6l3.5 3.5"/>',
  raise: '<path d="M10 16V5M6.5 8.5 10 5l3.5 3.5"/>',
  lower: '<path d="M10 4v11M6.5 11.5 10 15l3.5-3.5"/>',
  toBottom: '<path d="M4 17h12M10 4v10M6.5 10.5 10 14l3.5-3.5"/>',
  // Selection helpers.
  selectSame: `<path d="M3 4h6v6H3zM11 10h6v6h-6z" ${fillA}/><path d="M13 4h4v4h-4z"/>`,
  selectInvert: `<path d="M3 3h14v14H3z"/><path d="M3 3h7v14H3z" ${fillA}/>`,
  rulers: '<path d="M3 3h14v4H7v10H3z"/><path d="M9 3v2m3-2v2m3-2v2M3 9h2m-2 3h2m-2 3h2" />',
  guide: '<path d="M2 10h16" stroke-dasharray="3 2"/><path d="M10 2v16" stroke-dasharray="3 2" opacity=".5"/>',
};

/** An editor icon, or the app's icon of that name. */
export function svgIcon(name: string, size = 16): SVGSVGElement {
  const svg = icon(ICONS[name] ? 'more' : name, size);
  if (ICONS[name]) svg.innerHTML = ICONS[name];
  return svg;
}
