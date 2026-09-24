import type { Anchor, TransformSpec } from '../../style/svg/arrange';
import { h, type Child } from '../dom';
import { checkbox, numberInput, pair, row } from '../style/designerFields';
import type { EditActions } from './svgActions';
import type { SvgCanvas } from './svgCanvas';
import { liveSeg } from './svgStyleProps';
import { fmtNum } from './svgView';

/**
 * The Dönüştür tab of the SVG editor (Inkscape's Transform dialog): move
 * (by an amount or to a place), scale in %, rotate by an angle about the
 * centre, a box point or a point clicked on the canvas, skew, and a
 * matrix; for the whole selection or each shape on its own. Values stay
 * while the editor is open, so the same step can be applied again.
 */

const KINDS: { value: TransformSpec['kind']; label: string }[] = [
  { value: 'move', label: 'Taşı' },
  { value: 'scale', label: 'Ölçek' },
  { value: 'rotate', label: 'Döndür' },
  { value: 'skew', label: 'Eğ' },
  { value: 'matrix', label: 'Matris' },
];

const ANCHORS: Anchor[] = ['tl', 't', 'tr', 'l', 'c', 'r', 'bl', 'b', 'br'];
const ANCHOR_LABEL: Record<Anchor, string> = { tl: 'Sol üst', t: 'Üst orta', tr: 'Sağ üst', l: 'Sol orta', c: 'Merkez', r: 'Sağ orta', bl: 'Sol alt', b: 'Alt orta', br: 'Sağ alt' };

/** Nine buttons of the box's points (and a "picked point" one when asked). */
export function anchorPicker(value: Anchor | 'point', onChange: (a: Anchor) => void, label: string): HTMLElement {
  const grid = h('div', { class: 'svgp__anchors', role: 'radiogroup', 'aria-label': label });
  for (const a of ANCHORS) {
    const b = h('button', { class: 'svgp__anchor', type: 'button', role: 'radio', 'aria-checked': String(a === value), title: ANCHOR_LABEL[a], 'aria-label': ANCHOR_LABEL[a] });
    b.addEventListener('click', () => {
      onChange(a);
      grid.querySelectorAll('.svgp__anchor').forEach((x) => x.setAttribute('aria-checked', String(x === b)));
    });
    grid.append(b);
  }
  return grid;
}

export function transformTab(edit: EditActions, canvas: SvgCanvas, count: number, refresh: () => void): HTMLElement {
  const t = edit.ui.transform;
  const num = (label: string, value: number, set: (v: number) => void, unit?: string, step = 1) => row(label, numberInput(value, set, { label, unit, step }));
  const body: Child[] = [];
  switch (t.kind) {
    case 'move':
      body.push(
        liveSeg(
          'Taşıma biçimi',
          [
            { value: 'rel', label: 'Kadar', hint: 'Bu kadar kaydır' },
            { value: 'abs', label: 'Konuma', hint: 'Kutunun sol üst köşesi bu noktaya' },
          ],
          t.relative ? 'rel' : 'abs',
          (v) => ((t.relative = v === 'rel'), refresh()),
        ),
        pair(num(t.relative ? 'Yatay (X)' : 'X', t.x, (v) => (t.x = v)), num(t.relative ? 'Dikey (Y)' : 'Y', t.y, (v) => (t.y = v))),
      );
      break;
    case 'scale': {
      const sx = numberInput(t.sx, (v) => {
        t.sx = v;
        if (t.lock) {
          t.sy = v;
          (sy.querySelector('input') as HTMLInputElement).value = fmtNum(v);
        }
      }, { label: 'Genişlik', unit: '%', step: 10 });
      const sy = numberInput(t.sy, (v) => {
        t.sy = v;
        if (t.lock) {
          t.sx = v;
          (sx.querySelector('input') as HTMLInputElement).value = fmtNum(v);
        }
      }, { label: 'Yükseklik', unit: '%', step: 10 });
      body.push(pair(row('Genişlik', sx), row('Yükseklik', sy)), checkbox(t.lock, (v) => (t.lock = v), 'Oranı koru'), row('Sabit nokta', anchorPicker(t.anchor, (a) => (t.anchor = a), 'Ölçeğin sabit noktası')));
      break;
    }
    case 'rotate': {
      const pick = h('button', { class: 'btn btn--small', type: 'button', 'aria-pressed': String(t.about === 'point') }, t.point ? `Nokta: ${fmtNum(t.point[0])}, ${fmtNum(t.point[1])}` : 'Tuvalde göster…');
      pick.addEventListener('click', () =>
        canvas.pickPoint('Döndürme merkezini tuvalde tıklayın (kenetlenir); Esc vazgeçer.', (p) => {
          t.point = p;
          t.about = 'point';
          canvas.setMarker(p);
          refresh();
        }),
      );
      body.push(
        pair(
          num('Açı', t.deg, (v) => (t.deg = v), '°', 15),
          row(
            'Yön',
            liveSeg(
              'Dönme yönü',
              [
                { value: 'ccw', label: '↺', hint: 'Saat yönünün tersine' },
                { value: 'cw', label: '↻', hint: 'Saat yönünde' },
              ],
              t.ccw ? 'ccw' : 'cw',
              (v) => (t.ccw = v === 'ccw'),
            ),
          ),
        ),
        row(
          'Merkez',
          h(
            'div',
            { class: 'svgp__about' },
            anchorPicker(t.about, (a) => {
              t.about = a;
              canvas.setMarker(null);
              pick.setAttribute('aria-pressed', 'false');
            }, 'Döndürme merkezi'),
            pick,
          ),
          'Kutunun bir noktası ya da tuvalde tıklanan nokta',
        ),
      );
      break;
    }
    case 'skew':
      body.push(pair(num('Yatay eğim', t.ax, (v) => (t.ax = v), '°', 5), num('Dikey eğim', t.ay, (v) => (t.ay = v), '°', 5)), row('Sabit nokta', anchorPicker(t.anchor, (a) => (t.anchor = a), 'Eğmenin sabit noktası')));
      break;
    case 'matrix': {
      const m = t.m;
      const cell = (i: number, label: string) => row(label, numberInput(m[i], (v) => (m[i] = v), { label, step: 0.1 }));
      body.push(pair(cell(0, 'a'), cell(2, 'c')), pair(cell(1, 'b'), cell(3, 'd')), pair(cell(4, 'e'), cell(5, 'f')), h('div', { class: 'sdf__hint' }, "x' = a·x + c·y + e, y' = b·x + d·y + f (SVG matrix(a b c d e f))."));
      break;
    }
  }
  const apply = h('button', { class: 'btn btn--primary btn--small', type: 'button', disabled: !count }, 'Uygula');
  apply.addEventListener('click', () => edit.transform(specOf(edit), t.separately));
  return h(
    'div',
    { class: 'sdf__form' },
    h('div', { class: 'svgp__title' }, 'Dönüştür'),
    liveSeg('Dönüşüm', KINDS, t.kind, (v) => {
      t.kind = v;
      if (v !== 'rotate') canvas.setMarker(null);
      else if (t.about === 'point' && t.point) canvas.setMarker(t.point);
      refresh();
    }),
    h('div', { class: 'svgp__group' }, body),
    checkbox(t.separately, (v) => (t.separately = v), t.kind === 'move' ? 'Her birine ayrı (her şekil bir adım daha ileri)' : 'Her birine ayrı (kendi kutusuna göre)'),
    h('div', { class: 'svgp__foot' }, apply, h('span', { class: 'sdf__hint' }, count ? `${count} şekil` : 'Önce şekil seçin.')),
  );
}

export function specOf(edit: EditActions): TransformSpec {
  const t = edit.ui.transform;
  switch (t.kind) {
    case 'move':
      return { kind: 'move', x: t.x, y: t.y, relative: t.relative };
    case 'scale':
      return { kind: 'scale', sx: t.sx, sy: t.sy, anchor: t.anchor };
    case 'rotate':
      return { kind: 'rotate', deg: t.deg, ccw: t.ccw, about: t.about === 'point' ? (t.point ?? [0, 0]) : t.about };
    case 'skew':
      return { kind: 'skew', ax: t.ax, ay: t.ay, anchor: t.anchor };
    case 'matrix':
      return { kind: 'matrix', m: [...t.m] as [number, number, number, number, number, number] };
  }
}
