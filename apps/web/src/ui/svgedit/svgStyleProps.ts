import { boxToBox, rotation, shapesBox, transformShape, translate, type SvgShape } from '../../style/svg/svgModel';
import { h, type Child } from '../dom';
import { dashInput, numberInput, pair, row } from '../style/designerFields';
import { segmented, type Option } from '../widgets/controls';
import { svgIcon } from './svgIcons';
import { fmtNum } from './svgView';

/**
 * The stroke's look and the selection's box in the SVG editor's
 * properties: dash (presets scaled by the stroke width, or typed), line
 * ends, corners and fill rule; X, Y, width and height of the selection
 * (proportions locked on request) and a rotation by a typed angle.
 */

export interface StyleHost {
  readonly doc: { shapes: SvgShape[] };
  readonly selection: ReadonlySet<string>;
  change(key: string, fn: () => void): void;
}

const same = <T>(list: readonly T[]): T | null => (list.length && list.every((v) => v === list[0]) ? list[0] : null);

/** A segmented control that shows its new value at once (the panel is not redrawn while editing). */
export function liveSeg<T extends string>(label: string, options: Option<T>[], value: T | null, onChange: (v: T) => void): HTMLElement {
  const group = segmented({
    label,
    options,
    value: (value ?? '') as T,
    onChange: (v) => {
      onChange(v);
      group.querySelectorAll('.seg__opt').forEach((b, i) => {
        b.setAttribute('aria-checked', String(options[i].value === v));
        b.setAttribute('tabindex', options[i].value === v ? '0' : '-1');
      });
    },
  });
  group.classList.add('svgp__seg');
  return group;
}

const DASHES: { value: string; label: string; k: number[] | null }[] = [
  { value: 'solid', label: 'Sürekli', k: null },
  { value: 'dash', label: 'Kesikli', k: [3, 2] },
  { value: 'long', label: 'Uzun kesik', k: [6, 3] },
  { value: 'dot', label: 'Noktalı', k: [0.01, 2] },
  { value: 'dashdot', label: 'Kesik-nokta', k: [5, 2, 0.01, 2] },
  { value: 'custom', label: 'Özel', k: null },
];

export function strokeStyle(host: StyleHost, sel: SvgShape[]): HTMLElement {
  const set = (key: string, fn: (s: SvgShape) => SvgShape) =>
    host.change(key, () => {
      host.doc.shapes = host.doc.shapes.map((s) => (host.selection.has(s.id) ? fn(s) : s));
    });
  const width = same(sel.map((s) => s.strokeWidth)) ?? sel[0].strokeWidth;
  const dash = same(sel.map((s) => (s.dash ?? []).join(' ')));
  const preset = dash === '' ? 'solid' : (DASHES.find((d) => d.k && d.k.map((x) => fmtNum(x * width)).join(' ') === dash)?.value ?? 'custom');
  const typed = dashInput(dash ? dash.split(' ').map(Number) : null, (v) => set('dash', (s) => ({ ...s, dash: v ?? undefined })), 'Kesik deseni');
  const presets = h(
    'select',
    { class: 'field sdf__select', 'aria-label': 'Kesik deseni hazır' },
    DASHES.map((d) => h('option', { value: d.value, selected: d.value === preset, disabled: d.value === 'custom' }, d.label)),
  );
  presets.addEventListener('change', () => {
    const d = DASHES.find((x) => x.value === presets.value);
    if (!d || d.value === 'custom') return;
    // Presets follow the stroke width, as a pen's dashes do; dots need round ends.
    set('dash', (s) => ({ ...s, dash: d.k ? d.k.map((x) => Math.round(x * s.strokeWidth * 1000) / 1000) : undefined, ...(d.value === 'dot' || d.value === 'dashdot' ? { cap: 'round' as const } : {}) }));
    (typed as HTMLInputElement).value = d.k ? d.k.map((x) => fmtNum(x * width)).join(' ') : '';
  });
  const isPath = (s: SvgShape) => s.kind === 'path';
  const cap = same(sel.map((s) => s.cap ?? (isPath(s) ? 'round' : 'butt')));
  const join = same(sel.map((s) => s.join ?? (isPath(s) ? 'round' : 'miter')));
  const rule = same(sel.map((s) => s.fillRule ?? (isPath(s) ? 'evenodd' : 'nonzero')));
  const parts: Child[] = [
    h('div', { class: 'sdf__grouptitle' }, 'Çizgi biçimi'),
    row('Kesik', h('div', { class: 'svgp__dash' }, presets, typed), 'Boyları çizim biriminde: çizgi, boşluk … (hazırlar kalınlığa göre)'),
    row(
      'Uçlar',
      liveSeg(
        'Çizgi uçları',
        [
          { value: 'butt', label: 'Düz', hint: 'Uçta biter' },
          { value: 'round', label: 'Yuvarlak', hint: 'Yarım daire' },
          { value: 'square', label: 'Kare', hint: 'Yarım kalınlık uzar' },
        ],
        cap,
        (v) => set('cap', (s) => ({ ...s, cap: v })),
      ),
    ),
    row(
      'Köşeler',
      liveSeg(
        'Çizgi köşeleri',
        [
          { value: 'miter', label: 'Sivri', hint: 'Gönyeli köşe' },
          { value: 'round', label: 'Yuvarlak' },
          { value: 'bevel', label: 'Pah', hint: 'Kesik köşe' },
        ],
        join,
        (v) => set('join', (s) => ({ ...s, join: v })),
      ),
    ),
    row(
      'Dolgu kuralı',
      liveSeg(
        'Dolgu kuralı',
        [
          { value: 'evenodd', label: 'Tek-çift', hint: 'İç içe parçalar delik olur' },
          { value: 'nonzero', label: 'Sıfır olmayan', hint: 'Yönleri aynı parçalar dolu kalır' },
        ],
        rule,
        (v) => set('rule', (s) => ({ ...s, fillRule: v })),
      ),
    ),
  ];
  return h('div', { class: 'svgp__group' }, parts);
}

/** X, Y, width and height of the selection's box, and a rotation by a typed angle. */
export function boxFields(host: StyleHost, sel: SvgShape[]): HTMLElement {
  const b0 = shapesBox(sel)!;
  const cur = { x: b0.minX, y: b0.minY, w: b0.maxX - b0.minX, h: b0.maxY - b0.minY };
  let lock = false;
  const apply = (key: string, next: typeof cur) => {
    const b = shapesBox(host.doc.shapes.filter((s) => host.selection.has(s.id)));
    if (!b) return;
    const w0 = b.maxX - b.minX;
    const h0 = b.maxY - b.minY;
    const m =
      key === 'x' || key === 'y'
        ? translate(next.x - b.minX, next.y - b.minY)
        : boxToBox({ ...b, maxX: b.minX + Math.max(w0, 1e-9), maxY: b.minY + Math.max(h0, 1e-9) }, { minX: next.x, minY: next.y, maxX: next.x + Math.max(next.w, 1e-6), maxY: next.y + Math.max(next.h, 1e-6) });
    host.change(`box-${key}`, () => (host.doc.shapes = host.doc.shapes.map((s) => (host.selection.has(s.id) && !s.locked ? transformShape(s, m) : s))));
  };
  const inputOf = (wrap: HTMLElement) => wrap.querySelector('input')!;
  const xw = numberInput(cur.x, (v) => apply('x', { ...cur, x: (cur.x = v) }), { label: 'X', step: 1 });
  const yw = numberInput(cur.y, (v) => apply('y', { ...cur, y: (cur.y = v) }), { label: 'Y', step: 1 });
  const ww = numberInput(cur.w, (v) => {
    if (lock && cur.w > 0) {
      cur.h = (cur.h * v) / cur.w;
      inputOf(hw).value = fmtNum(cur.h);
    }
    cur.w = v;
    apply('w', cur);
  }, { label: 'Genişlik', step: 1, min: 0.001 });
  const hw = numberInput(cur.h, (v) => {
    if (lock && cur.h > 0) {
      cur.w = (cur.w * v) / cur.h;
      inputOf(ww).value = fmtNum(cur.w);
    }
    cur.h = v;
    apply('h', cur);
  }, { label: 'Yükseklik', step: 1, min: 0.001 });
  const lockBtn = h('button', { class: 'ibtn svgp__lock', type: 'button', 'aria-pressed': 'false', title: 'Oranı koru', 'aria-label': 'Oranı koru' }, svgIcon('unlock', 14));
  lockBtn.addEventListener('click', () => {
    lock = !lock;
    lockBtn.setAttribute('aria-pressed', String(lock));
    lockBtn.replaceChildren(svgIcon(lock ? 'lock' : 'unlock', 14));
  });
  let deg = 90;
  let turns = 0;
  const rot = numberInput(deg, (v) => (deg = v), { label: 'Döndürme açısı', unit: '°', step: 15 });
  const turn = (ccw: boolean) => {
    const b = shapesBox(host.doc.shapes.filter((s) => host.selection.has(s.id)));
    if (!b) return;
    const m = rotation(ccw ? -deg : deg, (b.minX + b.maxX) / 2, (b.minY + b.maxY) / 2);
    host.change(`rot${++turns}`, () => (host.doc.shapes = host.doc.shapes.map((s) => (host.selection.has(s.id) && !s.locked ? transformShape(s, m) : s))));
  };
  const turnBtn = (ccw: boolean) => {
    const b = h('button', { class: 'ibtn svgp__act', type: 'button', title: ccw ? 'Saat yönünün tersine döndür' : 'Saat yönünde döndür', 'aria-label': ccw ? 'Saat yönünün tersine döndür' : 'Saat yönünde döndür' }, ccw ? '↺' : '↻');
    b.addEventListener('click', () => turn(ccw));
    return b;
  };
  return h(
    'div',
    { class: 'svgp__group' },
    h('div', { class: 'sdf__grouptitle' }, 'Kutu'),
    pair(row('X', xw), row('Y', yw)),
    h('div', { class: 'svgp__wh' }, row('Genişlik', ww), lockBtn, row('Yükseklik', hw)),
    row('Döndür', h('div', { class: 'svgp__rot' }, rot, turnBtn(true), turnBtn(false))),
  );
}
