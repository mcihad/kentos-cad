import { pathDataOf, transformSubPaths, type SubPath } from '../../style/svg/pathData';
import { shapeId, type SvgShape } from '../../style/svg/svgModel';
import { TRACE_DEFAULTS, traceBitmap, type TraceOptions, type TraceResult } from '../../style/svg/trace';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { pickFile, readDataUrl, type FileHost } from './svgFile';
import { rasterize, type ReferenceLayer } from './svgReference';

/**
 * Trace bitmap (Inkscape's, simplified; pure part in style/svg/trace.ts):
 * the tracing reference or a picture file becomes filled paths with holes
 * in the symbol's colour. Brightness threshold, invert, speckle removal,
 * corner threshold, smoothing and simplification, with a live preview
 * over the picture. Traced from the reference, the paths land where the
 * reference lies; otherwise they are fitted into the canvas.
 */

const SVGNS = 'http://www.w3.org/2000/svg';
/** The last settings, for the next trace in this session. */
const last = { ...TRACE_DEFAULTS, maxSide: 600, separate: false };

export function openTraceDialog(host: FileHost, src: { reference?: ReferenceLayer; file?: File }): void {
  const o = { ...last };
  let href: string | null = src.reference?.ref?.href ?? null;
  let bitmap: ImageData | null = null;
  let result: TraceResult | null = null;
  let timer = 0;

  const view = document.createElementNS(SVGNS, 'svg') as SVGSVGElement;
  view.setAttribute('class', 'svgt__view');
  const stats = h('div', { class: 'svgt__stats', role: 'status' });
  const stage = h('div', { class: 'svgt__stage' });
  const form = h('div', { class: 'svgf__form' });
  const add = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, icon('check', 16), 'Çizime ekle');

  const choose = async () => {
    const f = await pickFile('image/png,image/jpeg,.svg,image/svg+xml');
    if (f) await load(f);
  };
  const load = async (f: File) => {
    href = /svg/i.test(f.type) || /\.svg$/i.test(f.name) ? await readDataUrl(new Blob([await f.text()], { type: 'image/svg+xml' })) : await readDataUrl(f);
    src = { file: f };
    await raster();
  };
  const raster = async () => {
    if (!href) {
      const pick = h('button', { class: 'btn', type: 'button' }, icon('fileOpen', 15), 'Görüntü seç…');
      pick.addEventListener('click', () => void choose());
      replaceChildren(stage, h('div', { class: 'svgt__empty' }, h('p', null, 'İzlenecek görüntü yok: bir PNG, JPEG ya da SVG seçin ya da önce altlık ekleyin.'), pick));
      return;
    }
    const r = await rasterize(href, o.maxSide);
    if (!r) return host.status('Görüntü okunamadı.', 'warn');
    bitmap = r.data;
    view.setAttribute('viewBox', `0 0 ${bitmap.width} ${bitmap.height}`);
    replaceChildren(stage, view);
    run();
  };
  const run = () => {
    if (!bitmap) return;
    result = traceBitmap(bitmap, o);
    const bm = bitmap;
    const img = document.createElementNS(SVGNS, 'image');
    img.setAttribute('href', href!);
    img.setAttribute('width', String(bm.width));
    img.setAttribute('height', String(bm.height));
    img.setAttribute('preserveAspectRatio', 'none');
    img.setAttribute('opacity', '0.3');
    const path = document.createElementNS(SVGNS, 'path');
    path.setAttribute('d', result.shapes.map((s) => pathDataOf([s.outer, ...s.holes], 2)).join(''));
    path.setAttribute('fill', host.options.ink);
    path.setAttribute('fill-rule', 'evenodd');
    path.setAttribute('fill-opacity', '0.85');
    view.style.background = host.options.paper;
    view.replaceChildren(img, path);
    stats.textContent = `${result.shapes.length} parça, ${result.holes} delik, ${result.nodes} düğüm${result.removed ? `; ${result.removed} benek atıldı` : ''} · ${bm.width} × ${bm.height} piksel`;
    add.disabled = !result.shapes.length;
  };
  const schedule = () => {
    Object.assign(last, o);
    clearTimeout(timer);
    timer = window.setTimeout(run, 60);
  };

  const slider = (label: string, key: keyof TraceOptions, min: number, max: number, step: number, show: (v: number) => string, hint: string, scale = 1) => {
    const value = (o[key] as number) * scale;
    const input = h('input', { type: 'range', class: 'svgt__range', min: String(min), max: String(max), step: String(step), value: String(value), 'aria-label': label });
    const out = h('span', { class: 'svgt__val num' }, show(value));
    input.addEventListener('input', () => {
      const v = Number(input.value);
      (o as Record<string, unknown>)[key] = v / scale;
      out.textContent = show(v);
      schedule();
    });
    return h('div', { class: 'svgf__field', title: hint }, h('span', { class: 'sdf__label' }, label), h('div', { class: 'svgt__slider' }, input, out), h('p', { class: 'sdf__hint' }, hint));
  };
  const check = (label: string, value: boolean, set: (v: boolean) => void) => {
    const cb = h('input', { type: 'checkbox', checked: value, 'aria-label': label });
    cb.addEventListener('change', () => (set(cb.checked), schedule()));
    return h('label', { class: 'sdf__check' }, cb, h('span', null, label));
  };
  const renderForm = () =>
    replaceChildren(
      form,
      slider('Parlaklık eşiği', 'threshold', 1, 254, 1, (v) => String(v), 'Bundan koyu pikseller mürekkeptir.'),
      check('Ters çevir (açık renkler mürekkep)', o.invert, (v) => (o.invert = v)),
      slider('Benek temizliği', 'speckle', 0, 400, 1, (v) => `${v} px²`, 'Bundan küçük lekeler ve delikler atılır.'),
      slider('Köşe eşiği', 'corner', 10, 170, 5, (v) => `${v}°`, 'Bundan keskin dönüşler köşe kalır, gerisi eğri olur.'),
      slider('Yumuşatma', 'smooth', 0, 100, 5, (v) => (v ? `%${v}` : 'kapalı'), 'Kapalıyken düz kenarlı çokgen; arttıkça daha az düğüm, daha yuvarlak eğri.', 100),
      slider('Sadeleştirme', 'tolerance', 0.2, 5, 0.1, (v) => `${v.toFixed(1)} px`, 'Çizginin pikselden en çok ne kadar sapabileceği.'),
      h(
        'div',
        { class: 'svgf__field' },
        h('span', { class: 'sdf__label' }, 'Çözünürlük (uzun kenar)'),
        segmented<string>({
          label: 'Çözünürlük',
          options: [
            { value: '300', label: '300 px' },
            { value: '600', label: '600 px' },
            { value: '1000', label: '1000 px' },
          ],
          value: String(o.maxSide),
          onChange: (v) => {
            o.maxSide = Number(v);
            Object.assign(last, o);
            renderForm();
            void raster();
          },
        }),
      ),
      check('Her parça ayrı şekil (grupta)', o.separate, (v) => (o.separate = v)),
    );

  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const another = h('button', { class: 'btn', type: 'button', title: 'Başka bir görüntü izle' }, icon('fileOpen', 15), 'Görüntü seç…');
  another.addEventListener('click', () => void choose());
  const dialog = new Dialog({
    title: 'Bitmap izle',
    width: 1000,
    className: 'dialog--svgfile dialog--svgtrace',
    content: [h('div', { class: 'svgf__cols svgt__cols' }, h('div', { class: 'svgt__left' }, stage, stats), form)],
    footer: [another, h('div', { class: 'dialog__foot-spacer' }), cancel, add],
    onClose: () => clearTimeout(timer),
    stack: true,
  });
  cancel.addEventListener('click', () => dialog.close());
  add.addEventListener('click', () => {
    if (!result || !bitmap) return;
    const ref = src.reference?.ref;
    const { width: W, height: H } = host.doc;
    // Where the pixels go: onto the reference, or fitted into the canvas.
    const k = ref ? ref.width / bitmap.width : Math.min(W / bitmap.width, H / bitmap.height);
    const x = ref ? ref.x : (W - bitmap.width * k) / 2;
    const y = ref ? ref.y : (H - bitmap.height * k) / 2;
    const m = [k, 0, 0, ref ? ref.height / bitmap.height : k, x, y] as const;
    const pieces: SubPath[][] = o.separate ? result.shapes.map((s) => [s.outer, ...s.holes]) : [result.shapes.flatMap((s) => [s.outer, ...s.holes])];
    const group = o.separate && pieces.length > 1 ? shapeId() : undefined;
    const shapes: SvgShape[] = pieces.map((subs) => ({ id: shapeId(), kind: 'path', subs: transformSubPaths(subs, m), fill: 'fill', stroke: 'none', strokeWidth: Math.max(1, W / 50), fillRule: 'evenodd', group, name: 'İz' }));
    host.change('trace', () => (host.doc.shapes = [...host.doc.shapes, ...shapes]));
    host.select(shapes.map((s) => s.id));
    host.status(`Bitmap izlendi: ${result.shapes.length} parça, ${result.holes} delik, ${result.nodes} düğüm. Ctrl+Z geri alır.`);
    dialog.close();
  });
  renderForm();
  if (src.file) void load(src.file);
  else void raster();
}
