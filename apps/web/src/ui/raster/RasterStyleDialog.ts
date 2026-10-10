import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import type { EditOperation } from '../../contracts/generated/EditOperation';
import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import { fixed } from '../../core/displayNumber';
import type { BandStats } from '../../io/rasterProtocol';
import { dimLabels } from '../../model/datasetLabels';
import type { RasterDataset, RasterEntity, RasterRender, RasterStretch, RasterStyle } from '../../model/entities';
import { cleanRasterStyle, RASTER_RAMPS, rasterKey, rasterProblem } from '../../model/rasterRules';
import { rasterService } from '../../render/rasterService';
import { uidOf, writeEdit } from '../../tools/editCommand';
import { h, replaceChildren } from '../dom';
import { checkField, field, summaryLine } from '../io/common';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { rampCss } from './ramps';

/**
 * Raster stili (docs/adr/0204 §4, §8; the desktop's `rasters/look.rs`): a raster's look edited and seen on the drawing
 * as it changes: how it is drawn (its colours, grey, its palette, a ramp, a shaded relief or a ramp lit by its relief),
 * its bands, the stretch with the bands' statistics, the ramp among wide samples of each, the light of the relief,
 * nodata, the transparency and the sampling. The drawing shows the look while the window is open, inside an undo group
 * that is let go (nothing recorded); Uygula writes it through `cad.entities.edit`'s `rasterStyle` as one undo step
 * (Raster stili), Vazgeç leaves the raster as it was. A NetCDF dataset's raster has its Veri seti section (docs/adr/0243
 * §11): each slice dimension's value, Zaman sürgüsünü izle, a mesh's lines and their colour.
 */
export function openRasterStyle(ctx: AppContext): void {
  const raster = [...ctx.selection.ids.value].map((id) => ctx.doc.get(id)).find((e): e is RasterEntity & { uid: string } => e?.kind === 'raster');
  if (!raster) {
    ctx.log.warn('Raster stili için önce bir raster seçin.');
    return;
  }
  if (ctx.doc.layers.isLocked(raster.layerId)) {
    ctx.log.warn(`“${ctx.doc.layers.get(raster.layerId)?.name ?? ''}” katmanı kilitli; rasterin görünüşü değiştirilemez.`);
    return;
  }
  new RasterStyleDialog(ctx, raster);
}

const RENDER_NAME: Record<RasterRender, string> = {
  rgb: 'Renkli (RGB)',
  gray: 'Gri',
  palette: 'Paletli',
  ramp: 'Renk rampası',
  hillshade: 'Gölgeli kabartma',
  rampShade: 'Rampa ve gölge',
};
const RENDERS: readonly RasterRender[] = ['rgb', 'gray', 'palette', 'ramp', 'hillshade', 'rampShade'];

type Typed = 'min' | 'max' | 'azimuth' | 'altitude' | 'zFactor' | 'nodata' | 'clear';

/** A typed number: undefined when blank, NaN when not a number. */
function read(text: string): number | undefined {
  const t = text.trim().replace(',', '.');
  if (!t) return undefined;
  const v = Number(t);
  return Number.isFinite(v) ? v : NaN;
}

const num = (v: number | null | undefined) => (v === null || v === undefined ? '' : String(v));

class RasterStyleDialog {
  private readonly ctx: AppContext;
  private readonly original: RasterEntity;
  private style: RasterStyle;
  /** The dataset being edited (a NetCDF raster's). */
  private dataset: RasterDataset | undefined;
  private edgesText: string;
  private readonly texts: Record<Typed, string>;
  private stats: BandStats[] | null | 'loading' = 'loading';
  private preview: { end(): void; cancel(): void } | null = null;
  private readonly body = h('div', { class: 'raster-style' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Uygula');
  private readonly dialog: Dialog;
  private applied = false;

  constructor(ctx: AppContext, raster: RasterEntity) {
    this.ctx = ctx;
    this.original = raster;
    this.style = { ...raster.style, bands: [...raster.style.bands] };
    this.dataset = raster.dataset ? (structuredClone(raster.dataset) as RasterDataset) : undefined;
    this.edgesText = raster.style.edges ?? '#2B3440';
    const s = raster.style;
    this.texts = {
      min: num(s.min),
      max: num(s.max),
      azimuth: num(s.azimuth),
      altitude: num(s.altitude),
      zFactor: num(s.zFactor),
      nodata: num(s.nodata),
      clear: String(Math.round((1 - (raster.opacity ?? 1)) * 100)),
    };
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'Raster stili',
      width: 620,
      className: 'dialog--io',
      content: [this.body],
      footer: [this.status, cancel, this.primary],
      onClose: () => {
        if (!this.applied) this.preview?.cancel();
        this.preview = null;
      },
    });
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => this.apply());
    this.render();
    const key = rasterKey(raster);
    const url = raster.asset ? (ctx.doc.styles.value.items.find((it) => it.id === raster.asset) as { data?: string } | undefined)?.data ?? null : null;
    void rasterService()
      .stats(key, url)
      .then((s) => (this.stats = s))
      .catch(() => (this.stats = null))
      .then(() => this.render());
  }

  /** The raster as the window says, when it is one. */
  private fields(): RasterEntity | null {
    const clear = read(this.texts.clear) ?? 0;
    if (!(Number.isFinite(clear) && clear >= 0 && clear <= 90)) return null;
    const opacity = 1 - clear / 100;
    const out: RasterEntity = { ...this.original, style: cleanRasterStyle(this.style) };
    if (this.dataset) out.dataset = this.dataset;
    if (opacity < 1) out.opacity = opacity;
    else delete out.opacity;
    return rasterProblem(out) === null ? out : null;
  }

  private problem(): string | null {
    const clear = read(this.texts.clear) ?? 0;
    if (!(Number.isFinite(clear) && clear >= 0 && clear <= 90)) return 'Saydamlık %0 ile %90 arasında olmalı.';
    if ((['min', 'max', 'azimuth', 'altitude', 'zFactor', 'nodata'] as const).some((k) => Number.isNaN(read(this.texts[k])))) return 'Sayı olmayan bir değer var; düzeltin ya da boş bırakın.';
    const f = { ...this.original, style: cleanRasterStyle(this.style), ...(this.dataset ? { dataset: this.dataset } : {}) };
    return rasterProblem(f);
  }

  /** The drawing shows the window's look: the last preview let go, the new one written in its group. */
  private showPreview(): void {
    this.preview?.cancel();
    this.preview = null;
    const f = this.fields();
    if (!f) return;
    const doc = this.ctx.doc;
    this.preview = doc.beginGroup('Raster stili');
    doc.update(this.original.id, { style: f.style, opacity: f.opacity, ...(f.dataset ? { dataset: f.dataset } : {}) } as never);
  }

  private apply(): void {
    const f = this.fields();
    if (!f) return;
    this.preview?.cancel();
    this.preview = null;
    this.applied = true;
    const same =
      JSON.stringify(cleanRasterStyle(this.original.style)) === JSON.stringify(f.style) &&
      (this.original.opacity ?? 1) === (f.opacity ?? 1) &&
      JSON.stringify(this.original.dataset ?? null) === JSON.stringify(f.dataset ?? null);
    this.dialog.close();
    if (same) return;
    const geometry = {
      kind: 'raster',
      affine: f.affine,
      width: f.width,
      height: f.height,
      bands: f.bands,
      sample: f.sample,
      ...(f.asset ? { asset: f.asset } : { file: f.file }),
      srid: f.srid,
      style: f.style,
      ...(f.opacity !== undefined && { opacity: f.opacity }),
      ...(f.dataset && { dataset: f.dataset }),
    } as unknown as EditGeometry;
    const out = writeEdit(this.ctx, 'rasterStyle' as EditOperation, [{ kind: 'update', uid: uidOf(this.ctx, f.id), geometry }]);
    if (out) {
      rasterService().forgetTiles();
      this.ctx.log.success('Rasterin görünüşü değişti. Ctrl+Z geri alır.');
    }
  }

  private changed(): void {
    this.showPreview();
    this.render();
  }

  private render(): void {
    const st = this.style;
    const bands = this.original.bands;
    const allowed = RENDERS.filter((r) => (r === 'rgb' ? bands >= 3 : r === 'palette' ? this.original.style.render === 'palette' : true));
    const render = h('select', { class: 'field', 'aria-label': 'Görünüş', dataset: { key: 'render' } }, allowed.map((r) => h('option', { value: r, selected: r === st.render }, RENDER_NAME[r])));
    render.addEventListener('change', () => {
      const r = render.value as RasterRender;
      st.render = r;
      st.bands = r === 'rgb' ? (bands >= 4 && this.original.sample === 'u8' ? [1, 2, 3, 4] : [1, 2, Math.min(3, bands)]) : [st.bands[0] ?? 1];
      if ((r === 'ramp' || r === 'rampShade') && !st.ramp) st.ramp = 'Arazi';
      this.changed();
    });
    const bandSelect = (k: number, none: boolean, label: string) => {
      const s = h(
        'select',
        { class: 'field', 'aria-label': label, dataset: { key: `band-${k}` } },
        none ? h('option', { value: '0', selected: st.bands[k] === undefined }, 'Yok') : null,
        Array.from({ length: bands }, (_, i) => h('option', { value: String(i + 1), selected: st.bands[k] === i + 1 }, `${i + 1}. bant`)),
      );
      s.addEventListener('change', () => {
        const b = Number(s.value);
        if (b === 0) st.bands = st.bands.slice(0, Math.min(k, 3));
        else if (k < st.bands.length) st.bands[k] = b;
        else if (k === st.bands.length) st.bands.push(b);
        this.changed();
      });
      return field(label, s);
    };
    const bandRow =
      st.render === 'rgb'
        ? h('div', { class: 'io-row' }, bandSelect(0, false, 'Kırmızı'), bandSelect(1, false, 'Yeşil'), bandSelect(2, false, 'Mavi'), bandSelect(3, true, 'Alfa'))
        : st.render === 'palette'
          ? h('p', { class: 'io-hint' }, 'Paletli raster kendi renkleriyle çizilir.')
          : bandSelect(0, false, 'Bant');
    const input = (k: Typed, label: string, placeholder: string) => {
      const i = h('input', { class: 'field raster-style__num', value: this.texts[k], placeholder, 'aria-label': label, spellcheck: 'false', dataset: { key: k } });
      i.addEventListener('input', () => {
        this.texts[k] = i.value;
        const v = read(i.value);
        if (k !== 'clear') (st as unknown as Record<string, number | undefined>)[k] = v;
        this.showPreview();
        this.renderStatus();
      });
      return field(label, i);
    };
    const parts: HTMLElement[] = [];
    if (this.dataset) parts.push(this.datasetPart(this.dataset));
    parts.push(h('div', { class: 'io-row' }, field('Görünüş', render), bandRow));
    if (st.render !== 'hillshade' && st.render !== 'palette') {
      const stretch = segmented<RasterStretch>({
        label: 'Gerdirme',
        options: [
          { value: 'none', label: 'Yok' },
          { value: 'minMax', label: 'En küçük – en büyük' },
          { value: 'percent', label: '%2 – %98' },
          { value: 'manual', label: 'Elle' },
        ],
        value: st.stretch ?? 'none',
        onChange: (v) => {
          st.stretch = v;
          this.changed();
        },
      });
      parts.push(field('Gerdirme', stretch, this.statsText()));
      if (st.stretch === 'manual') parts.push(h('div', { class: 'io-row' }, input('min', 'En küçük', 'en küçük'), input('max', 'En büyük', 'en büyük')));
    }
    if (st.render === 'ramp' || st.render === 'rampShade') {
      const list = h(
        'div',
        { class: 'raster-ramps', role: 'listbox', 'aria-label': 'Renk rampası' },
        RASTER_RAMPS.map((name) => {
          const row = h(
            'button',
            { class: 'raster-ramps__row', type: 'button', role: 'option', 'aria-selected': String((st.ramp ?? 'Gri') === name), dataset: { key: `ramp-${name}` } },
            h('span', { class: 'raster-ramps__bar', style: { background: rampCss(name, !!st.invert) } }),
            h('span', null, name),
          );
          row.addEventListener('click', () => {
            st.ramp = name;
            this.changed();
          });
          return row;
        }),
      );
      parts.push(
        field(
          'Renk rampası',
          h('div', null, list, checkField('', 'Ters çevir', !!st.invert, (v) => {
            st.invert = v;
            this.changed();
          }, 'invert')),
        ),
      );
    }
    if (st.render === 'hillshade' || st.render === 'rampShade')
      parts.push(
        field(
          'Gölgeli kabartma',
          h('div', { class: 'io-row' }, input('azimuth', 'Işığın doğrultusu (°)', '315'), input('altitude', 'Yüksekliği (°)', '45'), input('zFactor', 'Yükseklik çarpanı', '1')),
          'Işık kuzeyden saat yönünde ölçülür (315: kuzeybatı); kabartma Horn yöntemiyle.',
        ),
      );
    const sampling = segmented<'bilinear' | 'nearest'>({
      label: 'Örnekleme',
      options: [
        { value: 'bilinear', label: 'Çift doğrusal' },
        { value: 'nearest', label: 'En yakın' },
      ],
      value: st.resampling ?? 'bilinear',
      onChange: (v) => {
        st.resampling = v;
        this.changed();
      },
    });
    parts.push(h('div', { class: 'io-row' }, input('nodata', 'Nodata', 'dosyanınki'), input('clear', 'Saydamlık (%)', '0'), field('Örnekleme', sampling)));
    parts.push(this.statusLine);
    const focus = (document.activeElement as HTMLElement | null)?.dataset?.key;
    replaceChildren(this.body, parts);
    this.renderStatus();
    if (focus) this.body.querySelector<HTMLElement>(`[data-key="${focus}"]`)?.focus();
  }

  /** Veri seti (docs/adr/0243 §11): each slice dimension's value, Zaman sürgüsünü izle, a mesh's lines. */
  private datasetPart(d: RasterDataset): HTMLElement {
    const dims = d.dims ?? [];
    const rows: HTMLElement[] = [h('p', { class: 'io-value' }, d.vector ? `${d.variable} / ${d.vector} (vektör)` : d.variable)];
    if (dims.length)
      rows.push(
        h(
          'div',
          { class: 'io-row' },
          dims.map((x, k) => {
            const s = h(
              'select',
              { class: 'field', 'aria-label': x.time ? 'Zaman' : x.name, dataset: { key: `dim-${k}` } },
              dimLabels(x.values, !!x.time, x.units).map((l, i) => h('option', { value: String(i), selected: i === x.index }, l)),
            );
            s.addEventListener('change', () => {
              x.index = Number(s.value);
              this.changed();
            });
            return field(x.time ? 'Zaman' : x.name, s, undefined, 'grow');
          }),
        ),
      );
    if (dims.some((x) => x.time))
      rows.push(
        checkField('', 'Zaman sürgüsünü izle', !!d.followTime, (v) => {
          if (v) d.followTime = true;
          else delete d.followTime;
          this.changed();
        }, 'follow'),
      );
    if (d.mesh) {
      const colour = h('input', { class: 'field raster-style__num', value: this.edgesText, placeholder: '#2B3440', 'aria-label': 'Ağ çizgilerinin rengi', dataset: { key: 'edges-colour' } });
      colour.addEventListener('input', () => {
        this.edgesText = colour.value;
        if (this.style.edges != null) this.style.edges = colour.value.trim();
        this.showPreview();
        this.renderStatus();
      });
      rows.push(
        h(
          'div',
          { class: 'io-row' },
          checkField('', 'Ağ çizgileri', this.style.edges != null, (v) => {
            if (v) this.style.edges = this.edgesText.trim();
            else delete this.style.edges;
            this.changed();
          }, 'edges'),
          field('Renk', colour),
        ),
      );
    }
    return field('Veri seti', h('div', { class: 'io-stack' }, rows), 'Dilimin değerleri ve zaman sürgüsünü izleme çizimde hemen görünür; ağ çizgileri yüzlerin kenarlarıdır.');
  }

  private readonly statusLine = h('div', { class: 'io-summary' });

  private renderStatus(): void {
    const why = this.problem();
    this.primary.disabled = why !== null;
    replaceChildren(this.statusLine, why ? summaryLine('error', why) : summaryLine('info', 'Çizim pencerenin görünüşünü gösteriyor; Uygula tek adımda yazar, Vazgeç eski görünüşe döndürür.'));
  }

  /** The bands' statistics, as the stretch reads them. */
  private statsText(): string {
    const s = this.stats;
    if (s === 'loading') return 'Bantların istatistikleri hesaplanıyor…';
    if (!s) return 'İstatistik yok: rasterin dosyası bu oturumda okunamıyor.';
    const st = this.style;
    const bands = st.render === 'rgb' ? st.bands.slice(0, 3) : st.bands;
    // A whole number whole, else two decimals (the desktop's `stat_number`).
    const n = (v: number | null) => (v === null ? '—' : Number.isInteger(v) ? String(v) : fixed(v, 2));
    return bands
      .map((b) => {
        const x = s[b - 1];
        return x ? `${b}. bant: en küçük ${n(x.min)}, en büyük ${n(x.max)}; %2 ${n(x.low)}, %98 ${n(x.high)}` : '';
      })
      .filter(Boolean)
      .join(' · ');
  }
}
