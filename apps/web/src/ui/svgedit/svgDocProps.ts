import { shapesBox, transformShape } from '../../style/svg/svgModel';
import { numberInput, pair, row } from '../style/designerFields';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { scaleStroke, type SvgFiles } from './svgFile';

/**
 * Document properties (Inkscape's): the canvas as a window on the drawing
 * (viewBox: move it, crop or grow it, fit it to the content), a unit
 * scale for drawing and canvas together, the symbol's intended width in
 * mm ("1 birim = … mm", written into the file and used by exports), and
 * the preview's paper colour. One undo step.
 */

const n = (v: number) => String(Math.round(v * 1000) / 1000);

export function openDocProps(files: SvgFiles): void {
  const host = files.host;
  const doc = host.doc;
  const st = { x: 0, y: 0, w: doc.width, h: doc.height, scale: 1, margin: 0, mm: doc.sizeMm ?? 0, theme: !doc.background, paper: doc.background ?? host.options.paper };
  const form = h('div', { class: 'sdf__form svgf__form' });
  const result = h('p', { class: 'sdf__hint num' });

  const say = () => {
    const w = st.w * st.scale;
    const hgt = st.h * st.scale;
    result.textContent = `Yeni tuval: ${n(w)} × ${n(hgt)} birim${st.mm > 0 ? ` · 1 birim = ${n(st.mm / w)} mm · ${n(st.mm)} × ${n((st.mm * hgt) / w)} mm` : ''}`;
  };
  const num = (label: string, key: 'x' | 'y' | 'w' | 'h' | 'scale' | 'margin' | 'mm', opts: { min?: number; unit?: string; step?: number } = {}) =>
    row(label, numberInput(st[key], (v) => ((st[key] = v), say()), { label, min: opts.min, unit: opts.unit, step: opts.step ?? 1 }));
  const render = () => {
    const fit = h('button', { class: 'btn btn--small', type: 'button', title: 'Görünüm kutusunu çizimin sınırlarına (ve kenar payına) oturtur' }, icon('zoomExtents', 14), 'İçeriğe sığdır');
    fit.addEventListener('click', () => {
      const b = shapesBox(doc.shapes.filter((s) => !s.hidden));
      if (!b) return;
      const pad = Math.max(0, ...doc.shapes.map((s) => (s.stroke !== 'none' && !s.hidden ? s.strokeWidth / 2 : 0))) + st.margin;
      Object.assign(st, { x: b.minX - pad, y: b.minY - pad, w: b.maxX - b.minX + 2 * pad, h: b.maxY - b.minY + 2 * pad });
      render();
    });
    const theme = h('input', { type: 'checkbox', checked: st.theme, 'aria-label': 'Temanın kâğıdı' });
    theme.addEventListener('change', () => ((st.theme = theme.checked), render()));
    const picker = h('input', { type: 'color', class: 'svgp__color', value: st.paper.slice(0, 7), 'aria-label': 'Önizleme zemini' });
    picker.addEventListener('input', () => (st.paper = picker.value.toUpperCase()));
    replaceChildren(
      form,
      h('div', { class: 'sdf__grouptitle' }, 'Görünüm kutusu (tuval)'),
      pair(num('X', 'x'), num('Y', 'y')),
      pair(num('Genişlik', 'w', { min: 0.01 }), num('Yükseklik', 'h', { min: 0.01 })),
      h('div', { class: 'svgf__row' }, fit, num('Kenar payı', 'margin', { min: 0 })),
      h('p', { class: 'sdf__hint' }, 'Çizimin biriminde. Şekiller yerinde kalır; tuval bu pencereye kayar, kırpılır ya da büyür.'),
      h('div', { class: 'sdf__grouptitle' }, 'Ölçek ve sembol boyu'),
      pair(num('Birim ölçeği', 'scale', { min: 0.001, unit: '×', step: 0.1 }), num('Genişlik (mm)', 'mm', { min: 0, unit: 'mm', step: 0.5 })),
      h('p', { class: 'sdf__hint' }, 'Ölçek çizimi, tuvali ve çizgi kalınlıklarını birlikte büyütür (100 birim × 0.24 = 24 birim). Genişlik haritadaki boyudur (0: belirsiz); PNG’nin DPI’sı ve düz SVG’nin mm boyu buradan gelir.'),
      h('div', { class: 'sdf__grouptitle' }, 'Önizleme zemini'),
      h('div', { class: 'svgf__row' }, h('label', { class: 'sdf__check' }, theme, h('span', null, 'Temanın kâğıdı')), st.theme ? null : picker),
      result,
    );
    say();
  };

  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, icon('check', 16), 'Uygula');
  const dialog = new Dialog({ title: 'Belge özellikleri', width: 500, className: 'dialog--svgfile', content: [form], footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, ok], stack: true });
  cancel.addEventListener('click', () => dialog.close());
  ok.addEventListener('click', () => {
    const s = st.scale > 0 ? st.scale : 1;
    const m = [s, 0, 0, s, -st.x * s, -st.y * s] as const;
    const moves = st.x !== 0 || st.y !== 0 || s !== 1;
    host.change('docprops', () => {
      const d = host.doc;
      if (moves) d.shapes = d.shapes.map((sh) => scaleStroke(transformShape(sh, m), s));
      d.width = Math.max(0.01, st.w * s);
      d.height = Math.max(0.01, st.h * s);
      d.sizeMm = st.mm > 0 ? st.mm : undefined;
      d.background = st.theme ? undefined : st.paper;
    });
    // The tracing reference stays on the drawing.
    const ref = files.reference.ref;
    if (ref && moves) Object.assign(ref, { x: (ref.x - st.x) * s, y: (ref.y - st.y) * s, width: ref.width * s, height: ref.height * s });
    dialog.close();
    host.select([...host.selection]);
    host.canvas.fit();
    host.status(`Belge özellikleri uygulandı: ${n(host.doc.width)} × ${n(host.doc.height)} birim${host.doc.sizeMm ? `, ${n(host.doc.sizeMm)} mm` : ''}.`);
  });
  render();
}
