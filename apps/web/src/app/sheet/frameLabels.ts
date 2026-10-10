import type { MapPrim } from '../../contracts/generated/sheet/MapPrim';
import type { Entity } from '../../model/entities';
import { LABEL, LABEL_STRIDE } from '../../viewport/storeRecords';

/**
 * Which of the drawing's labels a map frame writes (decided for both
 * platforms, 3 Ekim): a label whose anchor is inside the frame's content is
 * written, and cut at the frame where it reaches past it; one whose anchor
 * is outside is not written, even where its letters would reach in. The same
 * records go to the screen's map frames, the gallery's pictures, the PNG and
 * SVG pictures and the PDF's vector texts (mapFrames.ts `labelSpots`).
 *
 * A label's anchor is the point it is written from:
 * - a text object's own point (`p`, whatever its alignment);
 * - an object's label (docs/adr/0212): its middle as the label engine placed it;
 * - any other (a dimension's value, a leader's note, a block's text): its record's point.
 */

/** Whether a ground point is inside a map frame's content: its view's centre, scale and turn over the frame's box. */
export function insideFrame(prim: Pick<MapPrim, 'clip' | 'view'>): (x: number, y: number) => boolean {
  const v = prim.view;
  const center = v.center;
  if (!center) return () => false;
  // Ground metres per micrometre of paper; the picture is turned by the view's turn (mapFrames.ts `draw`).
  const m = v.scale / 1_000_000;
  const hw = (prim.clip.width * m) / 2;
  const hh = (prim.clip.height * m) / 2;
  const turn = ((v.rotation % 360_000) * Math.PI) / 180_000;
  const c = Math.cos(turn);
  const s = Math.sin(turn);
  return (x, y) => {
    const dx = x - center.x;
    const dy = y - center.y;
    return Math.abs(dx * c + dy * s) <= hw && Math.abs(dx * s - dy * c) <= hh;
  };
}

/** A label record's anchor (see above); `e` is its object. */
export function labelAnchor(spots: Float64Array, i: number, e: Entity | undefined): [number, number] {
  const what = spots[i + 1];
  if (what === LABEL.text && e?.kind === 'text') return [e.p.x, e.p.y];
  return [spots[i + 2], spots[i + 3]];
}

/**
 * The records a frame writes: those of the objects `keep` passes whose anchor `inside` passes. An object's label as the
 * label engine placed it (docs/adr/0212 §3.8) goes whole or not at all, by its frame's middle.
 */
export function framedSpots(spots: Float64Array, object: (id: number) => Entity | undefined, inside: (x: number, y: number) => boolean, keep: (e: Entity) => boolean = () => true): Float64Array {
  const out: number[] = [];
  for (let i = 0; i + LABEL_STRIDE <= spots.length; i += LABEL_STRIDE) {
    const e = object(spots[i]);
    let end = i + LABEL_STRIDE;
    if (spots[i + 1] === LABEL.placed)
      while (end < spots.length && spots[end] === spots[i] && spots[end + 1] > LABEL.placed && spots[end + 1] <= LABEL.placedCallout) end += LABEL_STRIDE;
    if (e && keep(e)) {
      const [x, y] = labelAnchor(spots, i, e);
      if (inside(x, y)) for (let j = i; j < end; j++) out.push(spots[j]);
    }
    i = end - LABEL_STRIDE;
  }
  return Float64Array.from(out);
}
