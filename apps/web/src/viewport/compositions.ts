import type { Vec2 } from '../model/geometry';
import { BLEND_MODES } from '../model/layerComposeRules';
import type { LayerNode } from '../model/layers';
import type { LayerComposeFrame, MaskFrame } from '../render/types';
import { LABEL_STATE } from './placedLabels';
import { LABEL } from './storeRecords';

/**
 * A frame's compositions (docs/adr/0226 §3, §4), the desktop's `compositions` and `with_mask_labels`
 * (apps/desktop/src/style/scene.rs, viewport.rs): each shown layer's opacity, mode and the masks other shown layers'
 * Katman çizimi cuts into it with; a layer drawn as it always was is left out.
 */

/** CSS pixels a millimetre on a 96 dpi screen: a mask grows by its size there. */
const PX_PER_MM = 96 / 25.4;

/** The label masks a frame asks for: by the layer cut into, the layers whose placed labels cut it. */
export function frameCompositions(
  layer: (id: string) => LayerNode | undefined,
  order: readonly string[],
  opaque: boolean,
  labels: (wanted: ReadonlySet<string>) => ReadonlyMap<string, readonly number[]>,
): Map<string, LayerComposeFrame> | undefined {
  const sources = order.map(layer).filter((n): n is LayerNode => !!n?.compose?.mask);
  const wanted = new Set(sources.filter((n) => n.compose!.mask!.labels).map((n) => n.id));
  const boxes = wanted.size ? labels(wanted) : new Map<string, readonly number[]>();
  let out: Map<string, LayerComposeFrame> | undefined;
  for (const id of order) {
    const node = layer(id);
    if (!node) continue;
    const own = node.compose;
    // With Saydamlık off (docs/adr/0195) every layer is drawn opaque.
    const opacity = opaque ? 1 : Math.min(1, Math.max(0, own?.opacity ?? 1));
    const blend = own?.blend ? Math.max(0, BLEND_MODES.indexOf(own.blend)) : 0;
    const masks: MaskFrame[] = [];
    for (const src of sources) {
      const m = src.compose!.mask!;
      if (src.id === id || !m.targets.includes(id)) continue;
      masks.push({ markers: m.markers ? src.id : null, labels: m.labels ? (boxes.get(src.id) ?? []) : [], grow: m.size * PX_PER_MM });
    }
    if (opacity >= 1 && blend === 0 && !masks.length) continue;
    (out ??= new Map()).set(id, { opacity, blend, masks });
  }
  return out;
}

/**
 * The placed labels of the `wanted` layers as mask boxes, five numbers each: middle (metres from `origin`), width and
 * height (CSS px), turn (degrees, counter-clockwise); a curved label's letters. From the store's `labelsShown`
 * records (`stride` numbers each, a grown label's factor and anchor after its nine): hidden and unplaced labels are
 * no shapes.
 */
export function labelMaskBoxes(
  records: ArrayLike<number>,
  stride: number,
  layerOf: (id: number) => string | undefined,
  wanted: ReadonlySet<string>,
  origin: Vec2,
): Map<string, number[]> {
  const out = new Map<string, number[]>();
  const grown = stride >= 12;
  let skip = false;
  for (let i = 0; i + stride <= records.length; i += stride) {
    const what = records[i + 1];
    if (what !== LABEL.placed && what !== LABEL.placedLetter) continue;
    if (what === LABEL.placed) {
      const state = records[i + 8];
      // A curved label is its letters; a hidden or unplaced one, and its letters, is none.
      skip = (state & (LABEL_STATE.hidden | LABEL_STATE.unplaced)) !== 0;
      if (skip || (state & LABEL_STATE.curved) !== 0) continue;
    } else if (skip) continue;
    const layer = layerOf(records[i]);
    if (layer === undefined || !wanted.has(layer)) continue;
    const k = grown ? records[i + 9] || 1 : 1;
    const ax = grown ? records[i + 10] : records[i + 2];
    const ay = grown ? records[i + 11] : records[i + 3];
    const x = ax + (records[i + 2] - ax) * k;
    const y = ay + (records[i + 3] - ay) * k;
    const [w, h] = what === LABEL.placed ? [records[i + 5], records[i + 6]] : [records[i + 8], records[i + 5]];
    let list = out.get(layer);
    if (!list) out.set(layer, (list = []));
    list.push(x - origin.x, y - origin.y, w * k, h * k, records[i + 4]);
  }
  return out;
}
