import { sanitizeSvg } from '../../style/file';
import type { ReferenceSpec } from '../../style/svg/importSvg';
import { numberInput, pair, row } from '../style/designerFields';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { readDataUrl, type FileHost } from './svgFile';

/**
 * The tracing reference (izleme altlığı): a PNG, JPEG or SVG picture laid
 * under the drawing, above the paper, to redraw the regulation's raster
 * pictograms. Locked by default (clicks go through to the drawing);
 * unlocked it is dragged into place. Opacity, position, size and
 * visibility are set in the bar above the canvas; it is kept in the saved
 * file only when asked (in `<defs>`, so the symbol never draws it). Not
 * part of the drawing's undo: it is a working aid, like the grid.
 */

export interface Reference extends ReferenceSpec {
  visible: boolean;
  /** Written into the saved file. */
  keep: boolean;
}

const SVGNS = 'http://www.w3.org/2000/svg';
/** Above this the reference makes the library (kept in the browser) heavy. */
const HEAVY = 1.5e6;

export class ReferenceLayer {
  ref: Reference | null = null;
  /** The bar above the canvas (empty while there is no reference). */
  readonly bar: HTMLElement;
  private readonly host: FileHost;
  private readonly img: SVGImageElement;
  private readonly onTrace: () => void;
  private drag: { x: number; y: number; rx: number; ry: number; id: number } | null = null;

  constructor(host: FileHost, onTrace: () => void) {
    this.host = host;
    this.onTrace = onTrace;
    this.bar = h('div', { class: 'svgr', hidden: true });
    this.img = document.createElementNS(SVGNS, 'image') as SVGImageElement;
    this.img.setAttribute('preserveAspectRatio', 'none');
    this.img.setAttribute('class', 'svgr__img');
    this.img.addEventListener('pointerdown', (e) => {
      if (!this.ref || this.ref.locked || e.button !== 0) return;
      e.stopPropagation();
      this.img.setPointerCapture(e.pointerId);
      this.drag = { x: e.clientX, y: e.clientY, rx: this.ref.x, ry: this.ref.y, id: e.pointerId };
    });
    this.img.addEventListener('pointermove', (e) => {
      if (!this.drag || !this.ref) return;
      e.stopPropagation();
      const k = this.host.canvas.scale;
      this.ref.x = this.drag.rx + (e.clientX - this.drag.x) / k;
      this.ref.y = this.drag.ry + (e.clientY - this.drag.y) / k;
      this.place();
    });
    const end = (e: PointerEvent) => {
      if (!this.drag) return;
      e.stopPropagation();
      this.drag = null;
      this.renderBar();
    };
    this.img.addEventListener('pointerup', end);
    this.img.addEventListener('pointercancel', end);
  }

  /** Called by the canvas after the paper and grid, before the shapes. */
  underlay(world: SVGGElement): void {
    if (!this.ref?.visible) return;
    this.place();
    world.append(this.img);
  }

  private place(): void {
    const r = this.ref;
    if (!r) return;
    const img = this.img;
    if (img.getAttribute('href') !== r.href) img.setAttribute('href', r.href);
    img.setAttribute('x', String(r.x));
    img.setAttribute('y', String(r.y));
    img.setAttribute('width', String(r.width));
    img.setAttribute('height', String(r.height));
    img.setAttribute('opacity', String(r.opacity));
    img.setAttribute('pointer-events', r.locked ? 'none' : 'all');
    img.classList.toggle('svgr__img--free', !r.locked);
  }

  /** A picture file as the reference, fitted into the canvas. */
  async loadFile(f: File): Promise<void> {
    const isSvg = /svg/i.test(f.type) || /\.svg$/i.test(f.name);
    if (!isSvg && !/^image\/(png|jpeg)$/.test(f.type)) return this.host.status(`“${f.name}” altlık olamaz: PNG, JPEG ya da SVG seçin.`, 'warn');
    const href = isSvg ? await readDataUrl(new Blob([sanitizeSvg(await f.text())], { type: 'image/svg+xml' })) : await readDataUrl(f);
    const size = await naturalSize(href);
    if (!size) return this.host.status(`“${f.name}” açılamadı: görüntü okunamadı.`, 'warn');
    const { width: W, height: H } = this.host.doc;
    const k = Math.min(W / size.w, H / size.h);
    this.ref = { href, name: f.name.replace(/\.[a-z0-9]+$/i, ''), x: (W - size.w * k) / 2, y: (H - size.h * k) / 2, width: size.w * k, height: size.h * k, opacity: 0.5, locked: true, visible: true, keep: false };
    this.changed();
    this.host.status(`“${this.ref.name}” izleme altlığı oldu: kilitli ve yarı saydam. Üzerinden çizin ya da “İzle…” ile yola çevirin.`);
  }

  /** The reference kept in an opened file (or none). */
  restore(spec: ReferenceSpec | null): void {
    this.ref = spec ? { ...spec, visible: true, keep: true } : null;
    this.changed();
  }

  kept(): ReferenceSpec | null {
    const r = this.ref;
    return r?.keep ? { href: r.href, name: r.name, x: r.x, y: r.y, width: r.width, height: r.height, opacity: r.opacity, locked: r.locked } : null;
  }

  private changed(): void {
    this.host.canvas.render();
    this.renderBar();
  }

  private set(patch: Partial<Reference>): void {
    if (!this.ref) return;
    Object.assign(this.ref, patch);
    this.changed();
  }

  private fit(): void {
    const r = this.ref;
    if (!r) return;
    const { width: W, height: H } = this.host.doc;
    const k = Math.min(W / r.width, H / r.height);
    this.set({ width: r.width * k, height: r.height * k, x: (W - r.width * k) / 2, y: (H - r.height * k) / 2 });
  }

  renderBar(): void {
    const r = this.ref;
    this.bar.hidden = !r;
    if (!r) return replaceChildren(this.bar);
    const ib = (label: string, iconName: string, pressed: boolean | null, run: () => void) => {
      const b = h('button', { class: 'ibtn', type: 'button', title: label, 'aria-label': label, 'aria-pressed': pressed === null ? null : String(pressed) }, icon(iconName, 15));
      b.addEventListener('click', run);
      return b;
    };
    const opacity = h('input', { type: 'range', class: 'svgr__range', min: '5', max: '100', step: '5', value: String(Math.round(r.opacity * 100)), 'aria-label': 'Altlık saydamlığı' });
    const pct = h('span', { class: 'svgr__pct num' }, `%${Math.round(r.opacity * 100)}`);
    opacity.addEventListener('input', () => {
      r.opacity = Number(opacity.value) / 100;
      pct.textContent = `%${opacity.value}`;
      this.place();
    });
    const keep = h('input', { type: 'checkbox', checked: r.keep, 'aria-label': 'Dosyada sakla' });
    keep.addEventListener('change', () => {
      r.keep = keep.checked;
      if (r.keep && r.href.length > HEAVY) this.host.status(`Altlık büyük (${(r.href.length / 1e6).toFixed(1)} MB): kitaplık tarayıcıda tutulur, sakladığınız çizimler yer kaplar.`, 'warn');
      else this.host.status(r.keep ? 'Altlık kaydedince çizim dosyasında saklanır (sembolde çizilmez).' : 'Altlık dosyaya yazılmaz; pencere kapanınca gider.');
    });
    const small = (label: string, iconName: string, title: string, run: () => void) => {
      const b = h('button', { class: 'btn btn--small', type: 'button', title }, icon(iconName, 14), label);
      b.addEventListener('click', run);
      return b;
    };
    replaceChildren(
      this.bar,
      h('span', { class: 'svgr__name', title: r.name }, icon('layers', 14), `Altlık: ${r.name}`),
      ib(r.visible ? 'Altlığı gizle' : 'Altlığı göster', r.visible ? 'eye' : 'eyeOff', r.visible, () => this.set({ visible: !r.visible })),
      ib(r.locked ? 'Kilidi aç (sürükleyerek taşınır)' : 'Kilitle (tıklamalar çizime geçer)', r.locked ? 'lock' : 'unlock', r.locked, () => {
        this.set({ locked: !r.locked });
        this.host.status(r.locked ? 'Altlık kilitlendi.' : 'Altlık kilitsiz: boş yerinden sürükleyerek taşıyın; bitince kilitleyin.');
      }),
      h('label', { class: 'svgr__op', title: 'Saydamlık' }, 'Saydamlık', opacity, pct),
      small('Konum…', 'move', 'Konum ve boyut (X, Y, genişlik)', () => this.placeDialog()),
      small('Sığdır', 'zoomExtents', 'Tuvale sığdır (oran korunur)', () => this.fit()),
      h('label', { class: 'sdf__check svgr__keep', title: 'Kaydedince çizimle birlikte saklanır; sembolde çizilmez' }, keep, h('span', null, 'Dosyada sakla')),
      small('İzle…', 'spline', 'Altlığı delikli yollara çevir (Bitmap izle)', this.onTrace),
      ib('Altlığı kaldır', 'trash', null, () => {
        this.ref = null;
        this.changed();
      }),
    );
  }

  private placeDialog(): void {
    const r = this.ref;
    if (!r) return;
    const aspect = r.height / r.width;
    const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Tamam');
    const form = h(
      'div',
      { class: 'sdf__form svgf__form' },
      pair(row('X', numberInput(r.x, (v) => this.move({ x: v }), { label: 'X', step: 1 })), row('Y', numberInput(r.y, (v) => this.move({ y: v }), { label: 'Y', step: 1 }))),
      row('Genişlik', numberInput(r.width, (v) => this.move({ width: Math.max(0.01, v), height: Math.max(0.01, v) * aspect }), { label: 'Genişlik', min: 0.01, step: 1 }), 'Yükseklik oranla değişir. Birim çizimin birimidir.'),
    );
    const dialog = new Dialog({ title: 'Altlığın konumu ve boyu', width: 360, className: 'dialog--svgfile', content: [form], footer: [h('div', { class: 'dialog__foot-spacer' }), ok], stack: true });
    ok.addEventListener('click', () => (dialog.close(), this.renderBar()));
  }

  private move(patch: Partial<Reference>): void {
    if (!this.ref) return;
    Object.assign(this.ref, patch);
    this.host.canvas.render();
  }

  /** The reference as pixels for tracing: `maxSide` on its longer side. */
  async bitmap(maxSide: number): Promise<{ data: ImageData; image: HTMLImageElement } | null> {
    return this.ref ? rasterize(this.ref.href, maxSide) : null;
  }
}

async function naturalSize(href: string): Promise<{ w: number; h: number } | null> {
  const img = new Image();
  img.src = href;
  try {
    await img.decode();
  } catch {
    return null;
  }
  let w = img.naturalWidth;
  let h = img.naturalHeight;
  // An SVG with only a viewBox has no size of its own: its viewBox gives the proportions.
  if (href.startsWith('data:image/svg')) {
    const text = atob(href.slice(href.indexOf(',') + 1));
    const vb = /viewBox\s*=\s*["']\s*[-\d.e]+[\s,]+[-\d.e]+[\s,]+([\d.e]+)[\s,]+([\d.e]+)/i.exec(text);
    if (vb && (!w || !h || !/\swidth\s*=/.test(text.slice(0, text.indexOf('>'))))) {
      w = Number(vb[1]);
      h = Number(vb[2]);
    }
  }
  return w > 0 && h > 0 ? { w, h } : null;
}

/** A picture drawn into pixels on white, at most `maxSide` on its longer side. */
export async function rasterize(href: string, maxSide: number): Promise<{ data: ImageData; image: HTMLImageElement } | null> {
  const size = await naturalSize(href);
  if (!size) return null;
  const img = new Image();
  img.src = href;
  await img.decode();
  const k = Math.min(1, maxSide / Math.max(size.w, size.h)) || 1;
  // A vector picture is drawn at the working size; a raster one is not enlarged.
  const s = href.startsWith('data:image/svg') ? maxSide / Math.max(size.w, size.h) : k;
  const w = Math.max(1, Math.round(size.w * s));
  const hgt = Math.max(1, Math.round(size.h * s));
  const c = document.createElement('canvas');
  c.width = w;
  c.height = hgt;
  const g = c.getContext('2d', { willReadFrequently: true })!;
  g.drawImage(img, 0, 0, w, hgt);
  return { data: g.getImageData(0, 0, w, hgt), image: img };
}
