import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { EntityGeometry as NewGeometry } from '../../contracts/generated/EntityGeometry';
import { fixed } from '../../core/displayNumber';
import { crsBySrid, crsTitle } from '../../geo/crs';
import type { RasterFileInfo, RasterInspected, RasterPlacement } from '../../io/rasterProtocol';
import { crsSettings, ownSystem } from '../../model/projectCrs';
import { cleanRasterStyle } from '../../model/rasterRules';
import { entitiesCreate } from '../../product/entitiesCreate';
import { hasRaster } from '../../product/entitiesEdit';
import { sha256Hex, toBase64 } from '../../product/sheet/store';
import { rasterService } from '../../render/rasterService';
import { h, replaceChildren } from '../dom';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { checkField, field, fileLine, summaryLine } from '../io/common';
import { zoomToImported } from '../io/zoom';

/**
 * Raster ekle (docs/adr/0204 §1, §8; the desktop's `rasters/add.rs`): a GeoTIFF, TIFF, PNG or JPEG chosen with its
 * world file beside it if there is one (the browser never sees the folder: both are chosen together), its header read
 * in a raster worker from the file's slices (a TIFF's directories only); the window shows its size, bands, samples, how
 * it is stored, its levels, what places it, its system and nodata, and what the rule of the systems says
 * (`kentos_formats::raster::place`): the project's system, the user's yes for an unknown one, a refusal for another,
 * the middle of the view for an unplaced raster. Ekle writes a new layer named after the file and the raster on it as
 * one undo step (Raster ekle) and shows it. The file stays the session's (linked by its name); one of at most 32 MB
 * may be embedded instead, its bytes in the project's library.
 */
export function openRasterAdd(ctx: AppContext): void {
  void pick(ctx).then((files) => files && openFiles(ctx, files));
}

/** The window for the files: a NetCDF's variables and slices in their own (docs/adr/0243 §11). */
function openFiles(ctx: AppContext, files: File[]): RasterAddDialog | null {
  if (/\.nc$/i.test(files[0].name)) {
    void import('./MultidimDialog').then((m) => m.openNetcdf(ctx, files[0]));
    return null;
  }
  return new RasterAddDialog(ctx, files);
}

/** The files Raster ekle offers: rasters and their world files. */
export const RASTER_FILES: FileKind = {
  description: 'Raster (GeoTIFF, TIFF, PNG, JPEG, NetCDF) ve dünya dosyası',
  accept: {
    'application/x-netcdf': ['.nc'],
    'image/tiff': ['.tif', '.tiff'],
    'image/png': ['.png'],
    'image/jpeg': ['.jpg', '.jpeg'],
    'text/plain': ['.tfw', '.tifw', '.pgw', '.pngw', '.jgw', '.jpgw', '.jpegw', '.wld'],
  },
};

/** The largest file a raster is embedded from (docs/adr/0204 §2). */
export const MOST_EMBEDDED = 32 * 1024 * 1024;

async function pick(ctx: AppContext): Promise<File[] | null> {
  if (!ctx.doc) return null;
  const files = await ctx.files.pickFilesForImport(RASTER_FILES);
  if (!files?.length) return null;
  const image = files.find((f) => /\.(tiff?|png|jpe?g|nc)$/i.test(f.name));
  if (!image) {
    ctx.log.warn('Seçilen dosyalar arasında GeoTIFF, TIFF, PNG, JPEG ya da NetCDF yok; rasteri dünya dosyasıyla birlikte seçin.');
    return null;
  }
  return [image, ...files.filter((f) => f !== image)];
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const count = (n: number) => n.toLocaleString('tr-TR');

/** A file's size as the window says it. */
function sizeWords(bytes: number): string {
  return bytes >= 1 << 20 ? `${fixed(bytes / (1 << 20), 1).replace('.', ',')} MB` : `${Math.max(1, Math.floor(bytes / 1024))} KB`;
}

/** A raster's samples as the window names them. */
function colourWords(i: RasterFileInfo): string {
  const colour = i.color === 'rgb' ? 'RGB' : i.color === 'palette' ? 'paletli' : i.color === 'ycbcr' ? 'YCbCr (JPEG)' : 'gri';
  return `${i.bands} bant, ${sampleLabel(i.sample)}, ${colour}${i.alpha ? ', alfa bandıyla' : ''}`;
}

/** A sample type's name (the contract's `RasterSample::label`). */
export function sampleLabel(s: string): string {
  const LABELS: Record<string, string> = {
    u8: '8 bit',
    i8: '8 bit işaretli',
    u16: '16 bit',
    i16: '16 bit işaretli',
    u32: '32 bit',
    i32: '32 bit işaretli',
    f32: '32 bit kayan nokta',
    f64: '64 bit kayan nokta',
  };
  return LABELS[s] ?? s;
}

function storageWords(i: RasterFileInfo): string {
  if (!i.compression) return 'Tek parça resim';
  return `${i.compression}, ${i.tiled ? 'karolu' : 'şeritli'}${i.big ? ', BigTIFF' : ''}`;
}

/** A coordinate system as a sentence names it. */
function systemTitle(srid: number): string {
  const c = crsBySrid(srid);
  return c ? crsTitle(c) : `EPSG:${srid}`;
}

class RasterAddDialog {
  private readonly ctx: AppContext;
  private files: File[];
  private read: RasterInspected | null = null;
  private failed: string | null = null;
  private generation = 0;
  private closed = false;
  private confirmed = false;
  private embed = false;
  private readonly nameInput: HTMLInputElement;
  private readonly fileHost = h('div');
  private readonly factsHost = h('div', { class: 'io-summary raster-facts' });
  private readonly ruleHost = h('div', { class: 'io-summary' });
  private readonly optionsHost = h('div', { class: 'io-row' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Ekle');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, files: File[]) {
    this.ctx = ctx;
    this.files = files;
    this.nameInput = h('input', { class: 'field', value: stem(files[0].name), 'aria-label': 'Yeni katmanın adı', spellcheck: 'false', dataset: { key: 'name' } });
    this.nameInput.addEventListener('input', () => this.updateButton());
    this.nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && void this.run());
    const other = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Başka dosya…');
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'Raster ekle',
      width: 640,
      className: 'dialog--io',
      content: [this.fileHost, this.factsHost, this.ruleHost, this.optionsHost],
      footer: [other, this.status, cancel, this.primary],
      onClose: () => (this.closed = true),
    });
    other.addEventListener('click', () => void this.pickAnother());
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.run());
    void this.refresh();
  }

  /** The view's box, where an unplaced raster goes. */
  private view(): [number, number, number, number] {
    const b = this.ctx.view.camera.visibleBounds();
    return [b.minX, b.minY, b.maxX, b.maxY];
  }

  private async refresh(): Promise<void> {
    const gen = ++this.generation;
    this.read = null;
    this.failed = null;
    this.render();
    try {
      const r = await rasterService().inspect(this.files, this.ctx.doc.settings.crs.value.srid, false, this.view());
      if (gen !== this.generation || this.closed) return;
      this.read = r;
      this.embed = false;
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.failed = `Dosya okunamadı: ${message(e)}`;
    }
    this.render();
  }

  private async pickAnother(): Promise<void> {
    const files = await pick(this.ctx);
    if (!files || this.closed) return;
    if (/\.nc$/i.test(files[0].name)) {
      this.dialog.close();
      openFiles(this.ctx, files);
      return;
    }
    this.files = files;
    this.nameInput.value = stem(files[0].name);
    this.confirmed = false;
    void this.refresh();
  }

  private placement(): RasterPlacement | null {
    const r = this.read;
    if (!r) return null;
    return this.confirmed ? r.confirmedPlacement : r.placement;
  }

  private render(): void {
    const r = this.read;
    const file = this.files[0];
    const meta = r ? `${count(r.info.width)} × ${count(r.info.height)} piksel, ${sizeWords(file.size)}` : this.failed ? 'okunamadı' : 'okunuyor…';
    replaceChildren(this.fileHost, fileLine(file.name, meta));
    if (!r) {
      replaceChildren(this.factsHost, summaryLine(this.failed ? 'error' : 'info', this.failed ?? 'Dosyanın başlığı okunuyor…'));
      replaceChildren(this.ruleHost);
      replaceChildren(this.optionsHost);
      this.updateButton();
      return;
    }
    const i = r.info;
    const f = this.ctx.format;
    const levels = i.needsPyramid
      ? `${i.levels} kat; dosyada önizleme yok: ilk açılışta önizleme piramidi bir kez hazırlanır`
      : i.overviews > 0
        ? `${i.levels} kat; ${i.overviews} önizleme dosyada`
        : `${i.levels} kat`;
    const placed = i.placedBy === 'geotiff' ? 'GeoTIFF etiketleri' : i.placedBy === 'world' ? `Dünya dosyası${r.world ? ` (${r.world})` : ''}` : 'Yok (oturtulmamış)';
    const pixel = i.affine ? `${f.length(Math.hypot(i.affine[1], i.affine[4]), false)} × ${f.length(Math.hypot(i.affine[2], i.affine[5]), false)} ${f.lengthUnitLabel}` : '—';
    const system = i.epsg !== undefined ? systemTitle(i.epsg) : i.affine ? 'Dosya söylemiyor' : '—';
    const pairs: [string, string][] = [
      ['Bantlar', colourWords(i)],
      ['Saklama', storageWords(i)],
      ['Katlar', levels],
      ['Konum', placed],
      ['Piksel boyu', pixel],
      ['Sistem', system],
      ['Nodata', i.nodata !== undefined ? String(i.nodata) : 'Yok'],
    ];
    replaceChildren(this.factsHost, h('dl', { class: 'raster-facts__list' }, pairs.map(([k, v]) => [h('dt', null, k), h('dd', null, v)])));
    this.renderRule();
    this.renderOptions();
    this.updateButton();
  }

  private renderRule(): void {
    const r = this.read;
    if (!r) return;
    const own = ownSystem(crsSettings(this.ctx.doc.settings));
    const project = own ? own.title : 'Yerel (koordinat sistemi yok)';
    const p = r.placement;
    let line: HTMLElement;
    if (p.rule === 'same') line = summaryLine('ok', `Rasterin sistemi projeninkiyle aynı: ${project}. Dosyanın dediği yere eklenir.`);
    else if (p.rule === 'other')
      line = summaryLine('error', `Rasterin sistemi (${systemTitle(p.srid)}) projeninkinden (${project}) başka; raster eklenmez. KentOS rasteri yeniden izdüşürmez: rasteri projenin sistemine çevirip yeniden deneyin.`);
    else if (p.rule === 'unknown')
      line = summaryLine(
        'warn',
        'Dosya koordinat sistemini söylemiyor. Raster ancak projenin sisteminde olduğu söylenirse eklenir.',
        checkField('', `Raster projenin sisteminde (${project})`, this.confirmed, (v) => {
          this.confirmed = v;
          this.updateButton();
        }, 'confirm'),
      );
    else line = summaryLine('info', 'Rasterin konumu yok: görünümün ortasına, pikseli görünümün kısa kenarının binde biri olarak oturtulmamış eklenir; Raster oturt ile kontrol noktalarından yerine oturtulur.');
    replaceChildren(this.ruleHost, line);
  }

  private renderOptions(): void {
    const file = this.files[0];
    const large = file.size > MOST_EMBEDDED;
    const box = segmented({
      label: 'Kaynak',
      options: [
        { value: 'link', label: 'Bağlı' },
        { value: 'embed', label: 'Göm', disabled: large },
      ],
      value: this.embed ? 'embed' : 'link',
      onChange: (v) => {
        this.embed = v === 'embed';
        this.renderOptions();
      },
    });
    const hint = large
      ? `Dosya ${sizeWords(file.size)}; gömülü raster en çok ${MOST_EMBEDDED >> 20} MB olabilir, bağlı kalır.`
      : this.embed
        ? 'Dosyanın baytları projenin kitaplığına alınır; çizim dosyayla birlikte taşınır.'
        : 'Çizim dosyanın adını tutar; tarayıcı dosyayı bu oturum boyunca okur. Çizim yeniden açılınca dosya Öznitelikler’den yeniden seçilir.';
    replaceChildren(this.optionsHost, field('Kaynak', box, hint, 'grow'), field('Yeni katman', this.nameInput, 'Raster kendi katmanında durur.', 'grow'));
  }

  private updateButton(): void {
    const p = this.placement();
    this.primary.disabled = !p?.affine || !this.nameInput.value.trim();
  }

  private say(text: string, kind: 'info' | 'error' = 'info'): void {
    this.status.textContent = text;
    this.status.dataset.kind = kind;
  }

  private async run(): Promise<void> {
    const r = this.read;
    const p = this.placement();
    const name = this.nameInput.value.trim();
    if (this.primary.disabled || !r || !p?.affine || !name) return;
    const { ctx } = this;
    const file = this.files[0];
    let asset: string | undefined;
    let item: Record<string, unknown> | null = null;
    if (this.embed) {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const tiff = (bytes[0] === 0x49 && bytes[1] === 0x49) || (bytes[0] === 0x4d && bytes[1] === 0x4d);
      const format = tiff ? 'tiff' : bytes[0] === 0x89 ? 'png' : 'jpeg';
      asset = `raster-${(await sha256Hex(bytes)).slice(0, 16)}`;
      item = { kind: 'asset', id: asset, name: stem(file.name), path: ['Rasterler'], format, data: `data:image/${format};base64,${toBase64(bytes)}`, width: r.info.width, height: r.info.height };
    } else rasterService().setFile(file.name, file);
    const doc = ctx.doc;
    if (item && asset && !hasRaster(doc, asset)) doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, item as never] });
    const geometry = {
      kind: 'raster',
      affine: p.affine,
      width: r.info.width,
      height: r.info.height,
      bands: r.info.bands,
      sample: r.info.sample,
      ...(asset ? { asset } : { file: file.name }),
      srid: p.srid ?? 0,
      style: cleanRasterStyle(r.style),
    } as unknown as NewGeometry;
    let ids: number[] = [];
    let error: string | null = null;
    try {
      doc.transact('Raster ekle', () => {
        const layer = doc.addLayer({ name }, null, { activate: true });
        const result = entitiesCreate.execute({ doc }, { layerId: layer.id, objects: [{ geometry }], operation: 'raster' });
        if (result.status !== 'completed') {
          error = 'error' in result ? result.error.message : 'Raster yazılamadı.';
          throw new Error(error);
        }
        ids = result.output.ids;
        for (const w of result.warnings) ctx.log.warn(w.message);
      });
    } catch (e) {
      this.say(error ?? message(e), 'error');
      return;
    }
    zoomToImported(ctx, ids);
    ctx.log.success(`“${file.name}” ${asset ? 'gömülü' : 'bağlı'} raster olarak “${name}” katmanına eklendi. Ctrl+Z geri alır.`);
    if (!r.info.affine) ctx.log.info(`“${file.name}” oturtulmamış: Raster oturt ile kontrol noktalarından yerine oturtun.`);
    this.dialog.close();
  }
}

/** A file's stem. */
function stem(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(0, dot) : name;
}
