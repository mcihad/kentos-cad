import type { Entity } from './entities';
import type { LabelStyle } from './layers';

/**
 * Labels a layer without a label style gets, by kind: what the drawing, the sheet, Etiketleri yazıya çevir and the
 * linked texts the document keeps with their objects (docs/adr/0175 §4) read. The desktop's are the contract's
 * `default_label`; its app's tests read this literal (apps/desktop/src/labels.rs).
 */
export const DEFAULT_LABELS: Partial<Record<Entity['kind'], LabelStyle>> = {
  polygon: { placement: 'center', size: 10, grow: 1, maxSize: 14, minFeaturePx: 26 },
  circle: { placement: 'center', size: 10, minFeaturePx: 26 },
  point: { placement: 'beside', size: 10.5, minScale: 2 },
  polyline: { placement: 'along', size: 10, minScale: 1.6 },
  line: { placement: 'along', size: 10, minScale: 1.6 },
};
