import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import type { FileKind } from '../../app/fileIO';
import type { EntityGeometry as NewGeometry } from '../../contracts/generated/EntityGeometry';
import { crsBySrid, crsTitle } from '../../geo/crs';
import type { CubeDim, CubeInfo, CubePlaced } from '../../io/rasterProtocol';
import type { DatasetDim } from '../../model/entities';
import { crsSettings, ownSystem } from '../../model/projectCrs';
import { cleanRasterStyle } from '../../model/rasterRules';
import { readTime } from '../../model/time';
import { entitiesCreate } from '../../product/entitiesCreate';
import { hasRaster } from '../../product/entitiesEdit';
import { sha256Hex, toBase64 } from '../../product/sheet/store';
import { rasterService } from '../../render/rasterService';
import { h, replaceChildren } from '../dom';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { checkField, field, fileLine, select, summaryLine } from '../io/common';
import { zoomToImported } from '../io/zoom';
import { MOST_EMBEDDED, sampleLabel } from './RasterAddDialog';

/**
 * NetCDF and meshes added (docs/adr/0243 §11; the desktop's `rasters/multidim.rs`): Raster ekle hands a NetCDF file over
 * here, Mesh ekle opens it for a 2DM (and its ASCII DATs) or a UGRID NetCDF. The file is read in a raster worker: a
 * NetCDF's header, coordinates and meshes; a 2DM and its DATs made one UGRID file there (kept in the project's library
 * when it is small enough, else downloaded and the session's). The window lists the file's grids (Raster ekle) or its
 * meshes' datasets (Mesh ekle), a value of each slice dimension, Zaman sürgüsünü izle, a mesh's cell and lines, what the
 * rule of the systems says and how the file is kept; Ekle writes a new layer and the raster on it as one undo step.
 */

type Mode = 'grid' | 'mesh';

/** The files Mesh ekle offers. */
export const MESH_FILES: FileKind = {
  description: 'Mesh (2DM ve DAT, UGRID NetCDF)',
  accept: { 'application/x-netcdf': ['.nc'], 'text/plain': ['.2dm', '.dat'] },
};

/** Mesh ekle (`mesh.add`): the files chosen, then the window. */
export function openMeshAdd(ctx: AppContext): void {
  if (!ctx.doc) return;
  void ctx.files.pickFilesForImport(MESH_FILES).then((files) => files?.length && openMeshFiles(ctx, files));
}

/** Mesh ekle with the files given (a 2DM and its DATs, or a UGRID NetCDF): the window over them. */
export function openMeshFiles(ctx: AppContext, files: File[]): void {
  if (!ctx.doc || !files.length) return;
  const main = files.find((f) => /\.2dm$/i.test(f.name)) ?? files[0];
  const dats = /\.2dm$/i.test(main.name) ? files.filter((f) => /\.dat$/i.test(f.name)) : [];
  new MultidimDialog(ctx, 'mesh', main, dats);
}

/** Raster ekle's NetCDF: this window instead. */
export function openNetcdf(ctx: AppContext, file: File): void {
  new MultidimDialog(ctx, 'grid', file, []);
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const count = (n: number) => n.toLocaleString('tr-TR');

/** A variable the window offers: a grid by its place, or a mesh's (its place) dataset (its place). */
type Item = { grid: number } | { mesh: number; dataset: number };

function systemTitle(srid: number): string {
  const c = crsBySrid(srid);
  return c ? crsTitle(c) : `EPSG:${srid}`;
}

/** A name in the list: the shown name and the variable, the units; a vector and a face dataset said. */
function itemLabel(info: CubeInfo, item: Item): string {
  const named = (variable: string, long?: string, units?: string) => {
    let s = long && long !== variable ? `${long} (${variable})` : variable;
    if (units) s += `, ${units}`;
    return s;
  };
  if ('grid' in item) {
    const g = info.grids[item.grid];
    return named(g.variable, g.longName, g.units);
  }
  const d = info.meshes[item.mesh].datasets[item.dataset];
  let s = named(d.variable, d.longName, d.units);
  if (d.vector) s = `${s} / ${d.vector} (vektör)`;
  if (d.location === 'face') s += ' · yüzlerde';
  return s;
}

/** A file's stem. */
function stem(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(0, dot) : name;
}

class MultidimDialog {
  private readonly ctx: AppContext;
  private mode: Mode;
  /** The NetCDF the raster shows (a 2DM's written here). */
  private file: File;
  private readonly source: File;
  private readonly dats: File[];
  private info: CubeInfo | null = null;
  private report: { nodes: number; faces: number; datasets: string[]; notes: string[] } | null = null;
  private placed: CubePlaced | null = null;
  private failed: string | null = null;
  private placeFailed: string | null = null;
  private choice = 0;
  private slice: number[] = [];
  private follow = false;
  private cell = '';
  private edges = false;
  private start = '';
  private confirmed = false;
  private embed = false;
  private generation = 0;
  private closed = false;
  private readonly nameInput: HTMLInputElement;
  private readonly body = h('div', { class: 'io-stack' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Ekle');
  private readonly dialog: Dialog;

  constructor(ctx: AppContext, mode: Mode, file: File, dats: File[]) {
    this.ctx = ctx;
    this.mode = mode;
    this.file = file;
    this.source = file;
    this.dats = dats;
    this.nameInput = h('input', { class: 'field', value: stem(file.name), 'aria-label': 'Yeni katmanın adı', spellcheck: 'false', dataset: { key: 'name' } });
    this.nameInput.addEventListener('input', () => this.updateButton());
    this.nameInput.addEventListener('keydown', (e) => e.key === 'Enter' && void this.run());
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: mode === 'mesh' ? 'Mesh ekle' : 'Raster ekle',
      width: 640,
      className: 'dialog--io',
      content: [this.body],
      footer: [this.status, cancel, this.primary],
      onClose: () => (this.closed = true),
    });
    cancel.addEventListener('click', () => this.dialog.close());
    this.primary.addEventListener('click', () => void this.run());
    void this.read();
  }

  private isSms(): boolean {
    return /\.2dm$/i.test(this.source.name);
  }

  /** The view's box, where an unplaced raster goes. */
  private view(): [number, number, number, number] {
    const b = this.ctx.view.camera.visibleBounds();
    return [b.minX, b.minY, b.maxX, b.maxY];
  }

  /** The files read: a 2DM converted first, then what the NetCDF holds. */
  private async read(): Promise<void> {
    const gen = ++this.generation;
    this.info = null;
    this.placed = null;
    this.failed = null;
    this.render();
    try {
      if (this.isSms()) {
        const t = readTime(this.start);
        if (t.kind === 'unreadable') throw new Error('Başlangıç zamanı okunamadı: 2024-05-01 06:00 gibi yazın.');
        const out = await rasterService().sms(this.source, this.dats, t.kind === 'moment' ? t.t : Number.NaN, 0);
        this.report = out.report;
        this.file = new File([out.bytes as Uint8Array<ArrayBuffer>], `${stem(this.source.name)}.nc`, { type: 'application/x-netcdf' });
      }
      const info = await rasterService().cube(this.file, this.file.name);
      if (gen !== this.generation || this.closed) return;
      // A file of the other kind: the window lists what it has.
      if (this.mode === 'grid' && !info.grids.length && info.meshes.length) this.mode = 'mesh';
      else if (this.mode === 'mesh' && !info.meshes.length && info.grids.length) this.mode = 'grid';
      this.info = info;
      if (info.meshes[0]) this.cell = String(info.meshes[0].cell);
      this.embed = false;
      this.choice = Math.min(this.choice, Math.max(0, this.items().length - 1));
      this.resetSlice();
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.failed = message(e);
    }
    this.render();
    void this.place();
  }

  private items(): Item[] {
    const info = this.info;
    if (!info) return [];
    if (this.mode === 'grid') return info.grids.map((_, k) => ({ grid: k }));
    return info.meshes.flatMap((m, mk) => m.datasets.map((_, dk) => ({ mesh: mk, dataset: dk })));
  }

  private item(): Item | null {
    return this.items()[this.choice] ?? null;
  }

  private dims(): CubeDim[] {
    const info = this.info;
    const it = this.item();
    if (!info || !it) return [];
    return 'grid' in it ? info.grids[it.grid].dims : info.meshes[it.mesh].datasets[it.dataset].dims;
  }

  private resetSlice(): void {
    const dims = this.dims();
    this.slice = dims.map(() => 0);
    this.follow = dims.some((d) => d.time);
  }

  /** The part JSON of the chosen variable and slice (a mesh's grid made by the worker). */
  private part(): string | null {
    const info = this.info;
    const it = this.item();
    if (!info || !it) return null;
    if ('grid' in it) return JSON.stringify({ variable: info.grids[it.grid].variable, slice: this.slice });
    const m = info.meshes[it.mesh];
    const d = m.datasets[it.dataset];
    return JSON.stringify({ variable: d.variable, ...(d.vector ? { vector: d.vector } : {}), mesh: m.name, slice: this.slice });
  }

  /** The slice placed by the core's rule (re-asked when the variable, the cell or the lines change). */
  private async place(): Promise<void> {
    const part = this.part();
    if (!part) return;
    const gen = ++this.generation;
    this.placeFailed = null;
    const typed = Number(this.cell.trim().replace(',', '.'));
    const cell = this.mode === 'mesh' ? (this.cell.trim() && Number.isFinite(typed) ? typed : -1) : null;
    try {
      const placed = await rasterService().cubePlace(this.file, this.file.name, {
        part,
        cell,
        edges: this.edges,
        srid: this.ctx.doc.settings.crs.value.srid,
        confirmed: this.confirmed,
        view: this.view(),
      });
      if (gen !== this.generation || this.closed) return;
      this.placed = placed;
    } catch (e) {
      if (gen !== this.generation || this.closed) return;
      this.placed = null;
      this.placeFailed = message(e);
    }
    this.render();
  }

  private render(): void {
    const parts: (HTMLElement | null)[] = [];
    const info = this.info;
    let meta = this.failed ? 'okunamadı' : info ? (this.report ? `2DM, ${this.dats.length} DAT` : `NetCDF ${info.version}`) : 'okunuyor…';
    if (info?.meshes[0]) meta += ` · ${count(info.meshes[0].nodes)} düğüm, ${count(info.meshes[0].faces)} yüz`;
    parts.push(fileLine(this.source.name, meta));
    if (this.isSms()) parts.push(this.smsPart());
    if (!info) {
      parts.push(summaryLine(this.failed ? 'error' : 'info', this.failed ?? 'Dosya okunuyor…'));
    } else if (!this.items().length) {
      parts.push(
        h(
          'div',
          { class: 'io-summary' },
          summaryLine('error', this.mode === 'mesh' ? 'Dosyada okunabilen ağ yok.' : 'Dosyada okunabilen düzenli ızgara değişkeni yok.'),
          info.notes.slice(0, 6).map((n) => summaryLine('info', n)),
        ),
      );
    } else {
      parts.push(this.choicePart(info));
      if (this.placed) {
        parts.push(this.factsPart(this.placed));
        parts.push(this.rulePart(this.placed));
      } else if (this.placeFailed) parts.push(summaryLine('error', this.placeFailed));
      parts.push(this.optionsPart());
    }
    replaceChildren(this.body, parts);
    this.updateButton();
  }

  private smsPart(): HTMLElement {
    const start = h('input', { class: 'field', value: this.start, placeholder: '2024-05-01 06:00', 'aria-label': 'Başlangıç zamanı', dataset: { key: 'start' } });
    start.addEventListener('input', () => (this.start = start.value));
    start.addEventListener('keydown', (e) => e.key === 'Enter' && void this.read());
    const rows: HTMLElement[] = [
      h(
        'div',
        { class: 'io-row' },
        field('Başlangıç zamanı', start, "DAT'ın saatleri bu zamandan sayılır; boşsa RT_JULIAN, o da yoksa başlangıçtan saat. Enter uygular.", 'grow'),
        field('Yazılacak NetCDF', h('span', { class: 'io-value' }, `${stem(this.source.name)}.nc`), '2DM ve DAT\'lar bu UGRID dosyasına yazılır: projeye gömülür ya da indirilir.', 'grow'),
      ),
    ];
    if (this.report) {
      rows.push(
        h(
          'div',
          { class: 'io-summary' },
          summaryLine('ok', `${count(this.report.nodes)} düğüm, ${count(this.report.faces)} yüz; veri setleri: ${this.report.datasets.join(', ')}.`),
          this.report.notes.map((n) => summaryLine('info', n)),
        ),
      );
    }
    return h('div', { class: 'io-stack' }, rows);
  }

  private choicePart(info: CubeInfo): HTMLElement {
    const items = this.items();
    const list = select(
      this.mode === 'mesh' ? 'Veri seti' : 'Değişken',
      items.map((it, k) => ({ value: String(k), label: itemLabel(info, it) })),
      String(this.choice),
      (v) => {
        this.choice = Number(v);
        this.resetSlice();
        this.render();
        void this.place();
      },
      'variable',
    );
    const rows: HTMLElement[] = [field(this.mode === 'mesh' ? 'Veri seti' : 'Değişken', list, undefined, 'wide')];
    const dims = this.dims();
    if (dims.length) {
      rows.push(
        h(
          'div',
          { class: 'io-row' },
          dims.map((d, k) =>
            field(
              d.time ? 'Zaman' : d.name,
              select(
                d.time ? 'Zaman' : d.name,
                d.labels.map((l, i) => ({ value: String(i), label: l })),
                String(this.slice[k] ?? 0),
                (v) => (this.slice[k] = Number(v)),
                `dim-${k}`,
              ),
              undefined,
              'grow',
            ),
          ),
        ),
      );
    }
    if (dims.some((d) => d.time)) rows.push(checkField('', 'Zaman sürgüsünü izle', this.follow, (v) => (this.follow = v), 'follow'));
    if (this.mode === 'mesh') {
      const cell = h('input', { class: 'field', value: this.cell, placeholder: '0,5', 'aria-label': 'Hücre boyu', dataset: { key: 'cell' } });
      cell.addEventListener('change', () => {
        this.cell = cell.value;
        void this.place();
      });
      rows.push(
        h(
          'div',
          { class: 'io-row' },
          field('Hücre boyu (m)', cell, 'Mesh her katta bu ızgaranın hücreleriyle örneklenir; varsayılanı yüzlerin ortalama boyunun sekizde biri.', 'grow'),
          checkField('', 'Ağ çizgilerini çiz', this.edges, (v) => {
            this.edges = v;
            void this.place();
          }, 'edges'),
        ),
      );
    }
    return h('div', { class: 'io-stack' }, rows);
  }

  private factsPart(p: CubePlaced): HTMLElement {
    const i = p.info;
    const f = this.ctx.format;
    const pixel = i.affine ? `${f.length(Math.hypot(i.affine[1], i.affine[4]), false)} × ${f.length(Math.hypot(i.affine[2], i.affine[5]), false)} ${f.lengthUnitLabel}` : '—';
    const pairs: [string, string][] = [
      ['Izgara', `${count(i.width)} × ${count(i.height)} ${this.mode === 'mesh' ? 'hücre (sanal ızgara)' : 'hücre'}`],
      ['Hücre boyu', pixel],
      ['Örnekler', sampleLabel(i.sample)],
      ['Sistem', i.epsg !== undefined ? systemTitle(i.epsg) : 'Dosya söylemiyor'],
    ];
    return h('div', { class: 'io-summary raster-facts' }, h('dl', { class: 'raster-facts__list' }, pairs.map(([k, v]) => [h('dt', null, k), h('dd', null, v)])));
  }

  private rulePart(placed: CubePlaced): HTMLElement {
    const own = ownSystem(crsSettings(this.ctx.doc.settings));
    const project = own ? own.title : 'Yerel (koordinat sistemi yok)';
    const p = placed.placement;
    let line: HTMLElement;
    if (p.rule === 'same') line = summaryLine('ok', `Dosyanın sistemi projeninkiyle aynı: ${project}. Dosyanın dediği yere eklenir.`);
    else if (p.rule === 'other')
      line = summaryLine('error', `Dosyanın sistemi (${systemTitle(p.srid)}) projeninkinden (${project}) başka; eklenmez. KentOS yeniden izdüşürmez: veriyi projenin sistemine çevirip yeniden deneyin.`);
    else if (p.rule === 'unknown')
      line = summaryLine(
        'warn',
        'Dosya koordinat sistemini söylemiyor. Ancak projenin sisteminde olduğu söylenirse eklenir.',
        checkField('', `Veri projenin sisteminde (${project})`, this.confirmed, (v) => {
          this.confirmed = v;
          this.updateButton();
        }, 'confirm'),
      );
    else line = summaryLine('info', 'Izgaranın koordinatları yok: görünümün ortasına oturtulmamış eklenir; Raster oturt ile yerine oturtulur.');
    return h('div', { class: 'io-summary' }, line);
  }

  private optionsPart(): HTMLElement {
    const large = this.file.size > MOST_EMBEDDED;
    const box = segmented({
      label: 'Saklama',
      options: [
        { value: 'link', label: 'Bağlı', disabled: this.isSms() && !large },
        { value: 'embed', label: 'Göm', disabled: large },
      ],
      value: this.embed || (this.isSms() && !large) ? 'embed' : 'link',
      onChange: (v) => {
        this.embed = v === 'embed';
        this.render();
      },
    });
    const hint = large
      ? `Dosya ${count(Math.round(this.file.size / (1 << 20)))} MB; gömülü raster en çok ${MOST_EMBEDDED >> 20} MB olabilir, bağlı kalır.`
      : this.isSms()
        ? 'Yazılan NetCDF projenin kitaplığına alınır; çizim dosyayla birlikte taşınır.'
        : this.embed
          ? 'Dosyanın baytları projenin kitaplığına alınır; çizim dosyayla birlikte taşınır.'
          : 'Çizim dosyanın adını tutar; tarayıcı dosyayı bu oturum boyunca okur. Çizim yeniden açılınca dosya Öznitelikler’den yeniden seçilir.';
    return h('div', { class: 'io-row' }, field('Saklama', box, hint, 'grow'), field('Yeni katman', this.nameInput, 'Kendi katmanında durur.', 'grow'));
  }

  private placement() {
    const p = this.placed;
    if (!p) return null;
    return this.confirmed ? p.confirmedPlacement : p.placement;
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
    const placed = this.placed;
    const p = this.placement();
    const info = this.info;
    const it = this.item();
    const name = this.nameInput.value.trim();
    if (this.primary.disabled || !placed || !p?.affine || !info || !it || !name) return;
    const { ctx } = this;
    const file = this.file;
    const large = file.size > MOST_EMBEDDED;
    const embed = !large && (this.embed || this.isSms());
    let asset: string | undefined;
    if (embed) {
      const bytes = new Uint8Array(await file.arrayBuffer());
      asset = `raster-${(await sha256Hex(bytes)).slice(0, 16)}`;
      const doc = ctx.doc;
      if (!hasRaster(doc, asset))
        doc.styles.set({
          ...doc.styles.value,
          items: [
            ...doc.styles.value.items,
            { kind: 'asset', id: asset, name: stem(file.name), path: ['Rasterler'], format: 'netcdf', data: `data:application/x-netcdf;base64,${toBase64(bytes)}`, width: placed.info.width, height: placed.info.height } as never,
          ],
        });
    } else {
      rasterService().setFile(file.name, file);
      // A large 2DM's NetCDF is the session's: downloaded so that it can be given again.
      if (this.isSms()) {
        const url = URL.createObjectURL(file);
        const a = h('a', { href: url, download: file.name });
        document.body.append(a);
        a.click();
        a.remove();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      }
    }
    const part = JSON.parse(placed.part) as { variable: string; vector?: string; mesh?: string };
    const dims: DatasetDim[] = this.dims().map((d, k) => ({
      name: d.name,
      index: this.slice[k] ?? 0,
      values: d.values,
      ...(d.time ? { time: true } : {}),
      ...(d.units ? { units: d.units } : {}),
    }));
    const follow = this.follow && dims.some((d) => d.time);
    const geometry = {
      kind: 'raster',
      affine: p.affine,
      width: placed.info.width,
      height: placed.info.height,
      bands: 1,
      sample: placed.info.sample,
      ...(asset ? { asset } : { file: file.name }),
      srid: p.srid ?? 0,
      style: cleanRasterStyle(placed.style),
      dataset: {
        variable: part.variable,
        ...(part.vector ? { vector: part.vector } : {}),
        ...(part.mesh ? { mesh: part.mesh } : {}),
        ...(dims.length ? { dims } : {}),
        ...(follow ? { followTime: true } : {}),
      },
    } as unknown as NewGeometry;
    let ids: number[] = [];
    let error: string | null = null;
    const doc = ctx.doc;
    try {
      doc.transact(this.mode === 'mesh' ? 'Mesh ekle' : 'Raster ekle', () => {
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
    ctx.log.success(`“${file.name}” ${asset ? 'gömülü' : 'bağlı'} olarak “${name}” katmanına eklendi. Ctrl+Z geri alır.`);
    this.dialog.close();
  }
}
