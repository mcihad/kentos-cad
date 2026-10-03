import type { AnchorBox, AnchorsView, HAnchor, VAnchor } from '../../../product/sheet/view';

/**
 * The constraint editor's words and its pins' logic (docs/sheet/design.md
 * §3.2): which distance an item keeps on each axis when the paper changes,
 * what a click on a pin makes of it, and what a several-item choice shows
 * when its items differ (“—”). Apart from the DOM (ConstraintEditor.ts).
 */

export const H_TEXT: Record<HAnchor, { label: string; detail: string }> = {
  left: { label: 'Sol', detail: 'Sol kenara uzaklığı korunur.' },
  right: { label: 'Sağ', detail: 'Sağ kenara uzaklığı korunur.' },
  leftRight: { label: 'Sol ve sağ', detail: 'İki kenara uzaklığı korunur; öğe genişler ya da daralır.' },
  center: { label: 'Orta', detail: 'Ortasının kutunun ortasına uzaklığı korunur.' },
  scale: { label: 'Ölçekle', detail: 'Konumu ve genişliği kâğıtla oranlı değişir.' },
};

export const V_TEXT: Record<VAnchor, { label: string; detail: string }> = {
  top: { label: 'Üst', detail: 'Üst kenara uzaklığı korunur.' },
  bottom: { label: 'Alt', detail: 'Alt kenara uzaklığı korunur.' },
  topBottom: { label: 'Üst ve alt', detail: 'İki kenara uzaklığı korunur; öğe uzar ya da kısalır.' },
  center: { label: 'Orta', detail: 'Ortasının kutunun ortasına uzaklığı korunur.' },
  scale: { label: 'Ölçekle', detail: 'Konumu ve yüksekliği kâğıtla oranlı değişir.' },
};

export const BOX_TEXT: Record<AnchorBox, { label: string; detail: string }> = {
  page: { label: 'Sayfa', detail: 'Uzaklıklar kâğıdın kenarlarından ölçülür.' },
  margins: { label: 'Kenar boşlukları', detail: 'Uzaklıklar kenar boşluklarının çizgisinden ölçülür.' },
  group: { label: 'Grup', detail: 'Uzaklıklar öğenin grubunun çerçevesinden ölçülür.' },
};

export const H_ORDER: readonly HAnchor[] = ['left', 'right', 'leftRight', 'center', 'scale'];
export const V_ORDER: readonly VAnchor[] = ['top', 'bottom', 'topBottom', 'center', 'scale'];
export const BOX_ORDER: readonly AnchorBox[] = ['page', 'margins', 'group'];

/** One axis in neutral words: its start side (left, top), its end side, both, its middle, or scaled. */
export type AxisAnchor = 'start' | 'end' | 'both' | 'center' | 'scale';
export type Pin = 'start' | 'end' | 'center';

export const fromH: Record<HAnchor, AxisAnchor> = { left: 'start', right: 'end', leftRight: 'both', center: 'center', scale: 'scale' };
export const toH: Record<AxisAnchor, HAnchor> = { start: 'left', end: 'right', both: 'leftRight', center: 'center', scale: 'scale' };
export const fromV: Record<VAnchor, AxisAnchor> = { top: 'start', bottom: 'end', topBottom: 'both', center: 'center', scale: 'scale' };
export const toV: Record<AxisAnchor, VAnchor> = { start: 'top', end: 'bottom', both: 'topBottom', center: 'center', scale: 'scale' };

/**
 * What a click on a pin makes of an axis (Figma's way): a click takes that
 * side alone; with Shift a side joins the other one (both kept: the item
 * stretches) or leaves both; the middle line is the middle. A pin cannot be
 * clicked off: an axis always keeps something. A mixed axis (null) is taken
 * as nothing yet.
 */
export function clickPin(current: AxisAnchor | null, pin: Pin, extend: boolean): AxisAnchor {
  if (pin === 'center') return 'center';
  const other: Pin = pin === 'start' ? 'end' : 'start';
  if (!extend) return pin;
  if (current === other) return 'both';
  if (current === 'both') return other;
  return pin;
}

/** Whether a pin shows on (`true`), off, or on for some of the chosen items only (`mixed`). */
export function pinState(values: readonly (AxisAnchor | null)[], pin: Pin): boolean | 'mixed' {
  const on = (a: AxisAnchor | null) => a === pin || (a === 'both' && pin !== 'center');
  const n = values.filter(on).length;
  return n === 0 ? false : n === values.length ? true : 'mixed';
}

/** The anchors several items share, each field null where they differ (shown as “—”). */
export function commonAnchors(list: readonly AnchorsView[]): { h: HAnchor | null; v: VAnchor | null; box: AnchorBox | null } {
  const same = <T>(of: (a: AnchorsView) => T): T | null => (list.length && list.every((a) => of(a) === of(list[0])) ? of(list[0]) : null);
  return { h: same((a) => a.h), v: same((a) => a.v), box: same((a) => a.box) };
}

/** One line saying what the anchors do, for the inspector's hint (“Sağ ve alt kenara bağlı; kâğıtla birlikte kayar.”). */
export function anchorsSentence(h: HAnchor | null, v: VAnchor | null): string {
  if (h === null || v === null) return 'Seçili öğelerin kısıtları farklı: bir değer seçilince hepsine uygulanır.';
  if (h === 'scale' && v === 'scale') return 'Kâğıtla birlikte oranlı büyür ve küçülür.';
  if (h === 'leftRight' && v === 'topBottom') return 'Dört kenara uzaklığını korur: kâğıt büyüdükçe öğe de büyür.';
  if (h === 'center' && v === 'center') return 'Kâğıdın ortasında kalır.';
  const sides = [h === 'left' ? 'sol' : h === 'right' ? 'sağ' : null, v === 'top' ? 'üst' : v === 'bottom' ? 'alt' : null].filter(Boolean);
  if (sides.length) return `${sides.join(' ve ')} kenara bağlı kalır; kâğıt değişince oradan taşınır.`.replace(/^./, (c) => c.toLocaleUpperCase('tr-TR'));
  return 'Kâğıt değişince kısıtlarına göre yeniden yerleşir.';
}
