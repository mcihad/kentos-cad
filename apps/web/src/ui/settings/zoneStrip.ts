import { fixed } from '../../core/displayNumber';
import type { Province } from '../../geo/provinces';

/**
 * Türkiye's longitudes 25.5°–46.5° on a strip with the seven TUREF TM3 zones (docs/adr/0165 §3): the zone of the
 * chosen system lit, the province's place marked with its name. A click on a zone chooses it. The desktop draws the
 * same strip (project/wizard).
 */

const NS = 'http://www.w3.org/2000/svg';
/** The zones' central meridians, west to east; their TUREF systems are 5253 onwards. */
const MERIDIANS = [27, 30, 33, 36, 39, 42, 45];
const WEST = 25.5;
/** Units of the strip per degree, and its size. */
const PER_DEG = 30;
const W = 21 * PER_DEG;
const H = 64;

const x = (lon: number) => (lon - WEST) * PER_DEG;

function el<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number>, text?: string): SVGElementTagNameMap[K] {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  if (text !== undefined) e.textContent = text;
  return e;
}

/** The TUREF zone of an SRID (5253–5259) or the ED50 one (2319–2325), as an index west to east. */
function zoneIndex(srid: number | null): number | null {
  if (srid === null) return null;
  if (srid >= 5253 && srid <= 5259) return srid - 5253;
  if (srid >= 2319 && srid <= 2325) return srid - 2319;
  return null;
}

export function zoneStrip(o: { srid: number | null; province: Province | null; onPick?: (srid: number) => void }): SVGSVGElement {
  // Room at both ends for the outer longitudes' labels.
  const svg = el('svg', { viewBox: `-18 0 ${W + 36} ${H}`, class: 'zstrip', role: 'img', preserveAspectRatio: 'xMidYMid meet' });
  const chosen = zoneIndex(o.srid);
  svg.setAttribute('aria-label', `TM3 dilimleri, 25.5°–46.5° D${o.province ? `; ${o.province.name} ${fixed(o.province.lon, 2)}° D` : ''}`);
  MERIDIANS.forEach((cm, i) => {
    const g = el('g', { class: 'zstrip__zone', ...(i === chosen ? { 'data-chosen': '' } : {}) });
    g.append(el('rect', { x: x(cm - 1.5) + 1, y: 22, width: 3 * PER_DEG - 2, height: 24, rx: 3 }), el('text', { x: x(cm), y: 38, 'text-anchor': 'middle' }, `TM${cm}`));
    if (o.onPick) {
      g.addEventListener('click', () => o.onPick?.(5253 + i));
      g.style.cursor = 'pointer';
    }
    svg.append(g);
  });
  // The zone boundaries' longitudes under the strip.
  for (let k = 0; k <= 7; k++) {
    const lon = WEST + 3 * k;
    svg.append(el('text', { x: x(lon), y: 60, 'text-anchor': 'middle', class: 'zstrip__tick' }, `${lon}°`));
  }
  if (o.province) {
    const px = x(Math.min(46.5, Math.max(WEST, o.province.lon)));
    const anchor = px < 60 ? 'start' : px > W - 60 ? 'end' : 'middle';
    // The name and a pin above the strip, its point on the zone's edge: the zone's label stays clear.
    svg.append(
      el('text', { x: px, y: 9, 'text-anchor': anchor, class: 'zstrip__name' }, o.province.name),
      el('line', { x1: px, y1: 13, x2: px, y2: 20, class: 'zstrip__mark' }),
      el('path', { d: `M${px - 4} 17 L${px + 4} 17 L${px} 22 Z`, class: 'zstrip__dot' }),
    );
  }
  return svg;
}
