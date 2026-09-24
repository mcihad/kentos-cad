import { h, type Child } from '../dom';
import { checkbox, numberInput, pair, row } from '../style/designerFields';
import type { EditActions } from './svgActions';
import type { SvgCanvas } from './svgCanvas';
import { liveSeg } from './svgStyleProps';
import { fmtNum } from './svgView';

/**
 * The Dizi tab of the SVG editor, for pattern and symbol design (a CAD
 * array, Inkscape's tiled clones made simple): rows and columns by step or
 * gap, copies round a centre over a full turn or an arc (turning with it
 * or not), and a mirror copy across a vertical, horizontal or slanted
 * line. The copies show on the canvas as ghosts before "Uygula" makes them.
 */

export function arrayTab(edit: EditActions, canvas: SvgCanvas, count: number, refresh: () => void): HTMLElement {
  const a = edit.ui.array;
  // Every typed value redraws the ghosts at once.
  const num = (label: string, value: number, set: (v: number) => void, opts: { unit?: string; step?: number; min?: number } = {}) =>
    row(
      label,
      numberInput(
        value,
        (v) => {
          set(v);
          edit.previewArray();
        },
        { label, step: opts.step ?? 1, unit: opts.unit, min: opts.min },
      ),
    );
  const centre = (at: 'box' | 'canvas' | 'point', point: readonly [number, number] | null, set: (at: 'box' | 'canvas' | 'point', p?: readonly [number, number]) => void) => {
    const pick = h('button', { class: 'btn btn--small', type: 'button', 'aria-pressed': String(at === 'point') }, point && at === 'point' ? `Nokta: ${fmtNum(point[0])}, ${fmtNum(point[1])}` : 'Tuvalde göster…');
    pick.addEventListener('click', () =>
      canvas.pickPoint('Merkezi tuvalde tıklayın (kenetlenir); Esc vazgeçer.', (p) => {
        set('point', p);
        canvas.setMarker(p);
        edit.previewArray();
        refresh();
      }),
    );
    return row(
      'Merkez',
      h(
        'div',
        { class: 'svgp__about' },
        liveSeg(
          'Merkez',
          [
            { value: 'box', label: 'Seçim' },
            { value: 'canvas', label: 'Tuval' },
          ],
          at === 'point' ? null : at,
          (v) => {
            set(v);
            canvas.setMarker(null);
            pick.setAttribute('aria-pressed', 'false');
            edit.previewArray();
          },
        ),
        pick,
      ),
    );
  };
  const body: Child[] = [];
  if (a.kind === 'rect') {
    const r = a.rect;
    body.push(
      pair(num('Satır', r.rows, (v) => (r.rows = Math.max(1, Math.round(v))), { min: 1 }), num('Sütun', r.cols, (v) => (r.cols = Math.max(1, Math.round(v))), { min: 1 })),
      row(
        'Aralık',
        liveSeg(
          'Aralık biçimi',
          [
            { value: 'gap', label: 'Boşluk', hint: 'Kopyaların kutuları arası' },
            { value: 'step', label: 'Adım', hint: 'Bir kopyadan ötekine (merkezden merkeze)' },
          ],
          r.mode,
          (v) => ((r.mode = v), edit.previewArray()),
        ),
      ),
      pair(num('Yatay', r.dx, (v) => (r.dx = v), { step: 0.5 }), num('Dikey', r.dy, (v) => (r.dy = v), { step: 0.5 })),
    );
  } else if (a.kind === 'polar') {
    const p = a.polar;
    body.push(
      pair(num('Adet', p.count, (v) => (p.count = Math.max(1, Math.round(v))), { min: 1 }), num('Açı', p.angle, (v) => (p.angle = v), { unit: '°', step: 15 })),
      centre(p.at, p.point, (at, pt) => ((p.at = at), pt && (p.point = [pt[0], pt[1]]))),
      checkbox(p.rotate, (v) => ((p.rotate = v), edit.previewArray()), 'Kopyalar da dönsün'),
      row(
        'Yön',
        liveSeg(
          'Yön',
          [
            { value: 'ccw', label: '↺', hint: 'Saat yönünün tersine' },
            { value: 'cw', label: '↻', hint: 'Saat yönünde' },
          ],
          p.ccw ? 'ccw' : 'cw',
          (v) => ((p.ccw = v === 'ccw'), edit.previewArray()),
        ),
      ),
      h('div', { class: 'sdf__hint' }, '360°: tam tur, eşit aralık. Daha az: ilk ve son kopya yayın uçlarında.'),
    );
  } else {
    const m = a.mirror;
    body.push(
      row(
        'Eksen',
        liveSeg(
          'Aynalama ekseni',
          [
            { value: 'v', label: 'Dikey' },
            { value: 'h', label: 'Yatay' },
            { value: 'angle', label: 'Açılı' },
          ],
          m.axis,
          (v) => ((m.axis = v), edit.previewArray(), refresh()),
        ),
      ),
      m.axis === 'angle' ? num('Eksen açısı', m.deg, (v) => (m.deg = v), { unit: '°', step: 15 }) : null,
      centre(m.at, m.point, (at, pt) => ((m.at = at), pt && (m.point = [pt[0], pt[1]]))),
    );
  }
  const apply = h('button', { class: 'btn btn--primary btn--small', type: 'button', disabled: !count }, 'Uygula');
  apply.addEventListener('click', () => edit.applyArray());
  return h(
    'div',
    { class: 'sdf__form' },
    h('div', { class: 'svgp__title' }, 'Dizi ve aynalı kopya'),
    liveSeg(
      'Dizi türü',
      [
        { value: 'rect', label: 'Satır-sütun' },
        { value: 'polar', label: 'Dairesel' },
        { value: 'mirror', label: 'Aynalı' },
      ],
      a.kind,
      (v) => {
        a.kind = v;
        canvas.setMarker(null);
        edit.previewArray();
        refresh();
      },
    ),
    h('div', { class: 'svgp__group' }, body),
    checkbox(a.preview, (v) => ((a.preview = v), edit.previewArray()), 'Önizleme (kopyalar soluk görünür)'),
    h('div', { class: 'svgp__foot' }, apply, h('span', { class: 'sdf__hint' }, count ? `${count} şekil çoğaltılır` : 'Önce şekil seçin.')),
  );
}
