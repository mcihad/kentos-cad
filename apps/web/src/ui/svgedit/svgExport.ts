import { exportBox, pngSize, svgText, withPngDpi } from '../../style/svg/exportSvg';
import { numberInput } from '../style/designerFields';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { download, fileSlug, type FileHost } from './svgFile';

/**
 * The export window (Inkscape's Export dialog): a symbol SVG (the colour
 * parameters kept, as the library stores it), a plain SVG (the preview
 * colours written in, the size in mm, for other programs) or a PNG at a
 * pixel size or DPI on a transparent or paper background; the whole
 * drawing or only the selection; downloaded or copied to the clipboard.
 */

type Format = 'symbol' | 'plain' | 'png';

/** The last choices, for the next export in this session. */
const last = { format: 'symbol' as Format, only: false, by: 'px' as 'px' | 'dpi', px: 512, dpi: 300, paper: false };

const n = (v: number) => String(Math.round(v * 100) / 100);

/** A PNG of an SVG text at `w × h` pixels, on `background` or transparent. */
export async function renderPng(svg: string, w: number, h: number, background: string | null): Promise<Blob> {
  // The picture is drawn at its own size so the browser rasterises it sharp.
  const sized = svg.replace(/(<svg\b[^>]*?)\swidth="[^"]*"\sheight="[^"]*"/, `$1 width="${w}" height="${h}"`);
  const url = URL.createObjectURL(new Blob([sized], { type: 'image/svg+xml' }));
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    const g = c.getContext('2d')!;
    if (background) {
      g.fillStyle = background;
      g.fillRect(0, 0, w, h);
    }
    g.drawImage(img, 0, 0, w, h);
    return await new Promise<Blob>((ok, fail) => c.toBlob((b) => (b ? ok(b) : fail(new Error('PNG yazılamadı'))), 'image/png'));
  } finally {
    URL.revokeObjectURL(url);
  }
}

export function openExportDialog(host: FileHost): void {
  const sel = host.doc.shapes.filter((s) => host.selection.has(s.id) && !s.hidden);
  if (!host.doc.shapes.some((s) => !s.hidden)) return host.status('Dışa aktarılacak şekil yok: önce çizin ya da bir SVG açın.', 'warn');
  const st = { ...last, only: last.only && sel.length > 0 };
  const colors = { ink: host.options.ink, second: host.options.second };
  const body = h('div', { class: 'svgf__form' });
  const preview = h('img', { class: 'svgf__img', alt: 'Dışa aktarılacak çizim' });
  const info = h('div', { class: 'sdf__hint num' });
  const copy = h('button', { class: 'btn', type: 'button' }, icon('copy', 15), 'Panoya kopyala');
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, icon('export', 16), 'İndir');

  const only = () => (st.only ? new Set(sel.map((s) => s.id)) : undefined);
  const box = () => exportBox(host.doc, only());
  const widthMm = () => (host.doc.sizeMm ? (host.doc.sizeMm * box().w) / host.doc.width : undefined);
  const px = () => {
    const b = box();
    return pngSize(b.w, b.h, st.by === 'px' ? { px: st.px } : { dpi: st.dpi, widthMm: widthMm() });
  };
  const text = () => svgText(host.doc, { only: only(), colors: st.format === 'symbol' ? undefined : colors, pretty: true });
  const name = () => `${fileSlug(host.name)}${st.only ? '-secim' : ''}.${st.format === 'png' ? 'png' : 'svg'}`;
  const png = async () => {
    const { width, height } = px();
    const blob = await renderPng(svgText(host.doc, { only: only(), colors }), width, height, st.paper ? host.options.paper : null);
    const dpi = st.by === 'dpi' ? st.dpi : widthMm() ? (width / widthMm()!) * 25.4 : 96;
    return new Blob([withPngDpi(new Uint8Array(await blob.arrayBuffer()), dpi) as Uint8Array<ArrayBuffer>], { type: 'image/png' });
  };

  const render = () => {
    Object.assign(last, st);
    const b = box();
    const mm = widthMm();
    preview.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgText(host.doc, { only: only(), colors }))}`;
    preview.style.background = st.format === 'png' && !st.paper ? '' : host.options.paper;
    preview.classList.toggle('svgf__img--checker', st.format === 'png' && !st.paper);
    const size = st.format === 'png' ? (({ width, height }) => `${width} × ${height} piksel`)(px()) : `${n(b.w)} × ${n(b.h)} birim${mm ? ` · ${n(mm)} × ${n((mm * b.h) / b.w)} mm` : ''}`;
    info.textContent = `${name()} · ${size}`;
    const parts = [
      h(
        'div',
        { class: 'svgf__field' },
        h('span', { class: 'sdf__label' }, 'Biçim'),
        segmented<Format>({
          label: 'Biçim',
          options: [
            { value: 'symbol', label: 'SVG (sembol)', hint: 'Sembol rengi currentColor, ikinci renk param(stroke) kalır; kitaplığa ya da başka bir KentOS’a' },
            { value: 'plain', label: 'SVG (düz renk)', hint: 'Önizleme renkleri yazılır, boyut mm olarak; Inkscape ve öteki programlar için' },
            { value: 'png', label: 'PNG', hint: 'Piksel görüntü' },
          ],
          value: st.format,
          onChange: (v) => ((st.format = v), render()),
        }),
      ),
      h(
        'label',
        { class: 'sdf__check' },
        (() => {
          const cb = h('input', { type: 'checkbox', checked: st.only, disabled: !sel.length, 'aria-label': 'Yalnızca seçilenler' });
          cb.addEventListener('change', () => ((st.only = cb.checked), render()));
          return cb;
        })(),
        h('span', null, sel.length ? `Yalnızca seçilenler (${sel.length} şekil; tuval onlara kırpılır)` : 'Yalnızca seçilenler (seçim yok)'),
      ),
    ];
    if (st.format === 'png') {
      parts.push(
        h(
          'div',
          { class: 'svgf__field' },
          h('span', { class: 'sdf__label' }, 'Boyut'),
          segmented<'px' | 'dpi'>({
            label: 'Boyut',
            options: [
              { value: 'px', label: 'Piksel genişlik' },
              { value: 'dpi', label: 'DPI', hint: 'Belge özelliklerindeki sembol boyuyla (mm); yoksa 1 birim = 1 px (96 dpi)' },
            ],
            value: st.by,
            onChange: (v) => ((st.by = v), render()),
          }),
          st.by === 'px'
            ? numberInput(st.px, (v) => ((st.px = Math.max(1, Math.min(8192, Math.round(v)))), updateInfo()), { label: 'Piksel genişlik', unit: 'px', min: 1, max: 8192, step: 64 })
            : numberInput(st.dpi, (v) => ((st.dpi = Math.max(1, Math.min(2400, v))), updateInfo()), { label: 'DPI', unit: 'dpi', min: 1, max: 2400, step: 50 }),
          st.by === 'dpi' && !host.doc.sizeMm ? h('p', { class: 'sdf__hint' }, 'Çizimin mm boyu yok: Dosya → Belge özellikleri’nden sembol boyunu verin; şimdilik 1 birim = 1 px (96 dpi).') : null,
        ),
        h(
          'div',
          { class: 'svgf__field' },
          h('span', { class: 'sdf__label' }, 'Zemin'),
          segmented<'clear' | 'paper'>({
            label: 'Zemin',
            options: [
              { value: 'clear', label: 'Saydam' },
              { value: 'paper', label: 'Kâğıt', hint: 'Önizleme zemininin rengi' },
            ],
            value: st.paper ? 'paper' : 'clear',
            onChange: (v) => ((st.paper = v === 'paper'), render()),
          }),
        ),
      );
    }
    replaceChildren(body, parts);
  };
  const updateInfo = () => {
    Object.assign(last, st);
    const { width, height } = px();
    info.textContent = `${name()} · ${width} × ${height} piksel`;
  };

  const cancel = h('button', { class: 'btn', type: 'button' }, 'Kapat');
  const dialog = new Dialog({
    title: 'Dışa aktar',
    width: 760,
    className: 'dialog--svgfile',
    content: [h('div', { class: 'svgf__cols' }, h('div', { class: 'svgf__preview' }, preview, info), body)],
    footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, copy, save],
    stack: true,
  });
  cancel.addEventListener('click', () => dialog.close());
  save.addEventListener('click', async () => {
    try {
      download(st.format === 'png' ? await png() : new Blob([text()], { type: 'image/svg+xml' }), name());
      host.status(`${name()} indirildi.`);
      dialog.close();
    } catch (e) {
      host.status(`Dışa aktarılamadı: ${(e as Error).message}`, 'warn');
    }
  });
  copy.addEventListener('click', async () => {
    try {
      if (st.format === 'png') await navigator.clipboard.write([new ClipboardItem({ 'image/png': await png() })]);
      else await navigator.clipboard.writeText(text());
      host.status(st.format === 'png' ? 'PNG panoya kopyalandı.' : 'SVG metni panoya kopyalandı: başka bir çizime Ctrl+V ile eklenir.');
      dialog.close();
    } catch {
      host.status('Tarayıcı panoya yazmaya izin vermedi; İndir ile kaydedin.', 'warn');
    }
  });
  render();
}
