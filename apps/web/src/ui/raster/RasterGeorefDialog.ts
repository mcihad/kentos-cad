import type { AppContext } from '../../app/context';
import type { EditOperation } from '../../contracts/generated/EditOperation';
import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import { fixed } from '../../core/displayNumber';
import type { RasterEntity } from '../../model/entities';
import { rasterKey, cleanRasterStyle } from '../../model/rasterRules';
import { hasRaster } from '../../product/entitiesEdit';
import { sha256Hex, toBase64 } from '../../product/sheet/store';
import { rasterService } from '../../render/rasterService';
import { PickPointTool } from '../../tools/pickPointTool';
import { uidOf, writeEdit } from '../../tools/editCommand';
import { op } from '../../wasm/core';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { checkField, select } from '../io/common';
import { download } from '../svgedit/svgFile';
import { segmented, textField } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { field, Grid, readNumber, summary, summaryLine, type GridModel, type Row } from '../calc/common';
import { MOST_EMBEDDED } from './RasterAddDialog';

/**
 * Raster oturt (docs/adr/0204 §6; the desktop's `calc/raster_fit.rs`): a raster fitted to the drawing by control
 * points. Each row is a pixel of the raster (column and row, shown on the raster where it is drawn) and where it is to
 * go (Y and X, shown on the drawing, snapped, or typed); Kullan leaves a row out of the solution. The transform is the
 * geometry core's (`ops::georef`, through WASM): Helmert, affine, projective, polynomials of order 2 and 3, thin plate;
 * every row's residual (metres) and m0, solved again at every change. Uygula: Helmert and affine set the raster's
 * affine (one step, Raster oturt), with its world file downloaded when asked; the others resample the raster in its
 * raster worker into a tiled GeoTIFF (`<ad>-oturtulmus.tif`), which is downloaded and, at most 32 MB, embedded; else
 * it stays the session's file. What is typed stays for the session.
 */
export function openRasterGeoref(ctx: AppContext): void {
  new RasterGeorefDialog(ctx);
}

type Method = 'helmert' | 'affine' | 'projective' | 'poly2' | 'poly3' | 'thinPlate';
const METHODS: { value: Method; label: string }[] = [
  { value: 'helmert', label: 'Helmert' },
  { value: 'affine', label: 'Afin' },
  { value: 'projective', label: 'Projektif' },
  { value: 'poly2', label: 'Polinom 2' },
  { value: 'poly3', label: 'Polinom 3' },
  { value: 'thinPlate', label: 'İnce plaka' },
];
const NEED: Record<Method, number> = { helmert: 2, affine: 3, projective: 4, poly2: 6, poly3: 10, thinPlate: 3 };
const HINT: Record<Method, string> = {
  helmert: 'Öteler, döndürür, ölçekler; en az 2 nokta. Rasterin dönüşümü değişir, yeniden örneklenmez.',
  affine: 'İki yönde ayrı ölçek ve kayma da; en az 3 nokta. Rasterin dönüşümü değişir, yeniden örneklenmez.',
  projective: 'Eğik çekilmiş fotoğraf için; en az 4 nokta. Raster yeniden örneklenir.',
  poly2: 'Hafif bükülmeler için; en az 6 nokta. Raster yeniden örneklenir.',
  poly3: 'Daha güçlü bükülmeler için; en az 10 nokta. Raster yeniden örneklenir.',
  thinPlate: 'Noktalardan tam geçer (artık yok); en az 3 nokta. Raster yeniden örneklenir.',
};
const NAME: Record<Method, string> = { helmert: 'Helmert', affine: 'Afin', projective: 'Projektif', poly2: 'Polinom 2', poly3: 'Polinom 3', thinPlate: 'İnce plaka' };
const affineMethod = (m: Method) => m === 'helmert' || m === 'affine';

const COLUMNS = [
  { key: 'use', label: 'Kullan', check: true },
  { key: 'col', label: 'Sütun', unit: 'piksel', numeric: true },
  { key: 'row', label: 'Satır', unit: 'piksel', numeric: true },
  { key: 'ty', label: 'Hedef Y', numeric: true },
  { key: 'tx', label: 'Hedef X', numeric: true },
  { key: 'vy', label: 'vY', unit: 'm', numeric: true },
  { key: 'vx', label: 'vX', unit: 'm', numeric: true },
  { key: 'v', label: 'v', unit: 'm', numeric: true },
];
const RESIDUALS = new Set(['vy', 'vx', 'v']);

const state = {
  raster: null as number | null,
  method: 'affine' as Method,
  rows: [{}, {}, {}, {}] as Row[],
  pixel: '',
  nearest: false,
  world: false,
};

interface Gcp {
  pixel: { x: number; y: number };
  target: { x: number; y: number };
  used: boolean;
}

type Answer = { residuals: [number, number, number][]; m0: number | null; affine: number[] | null } | { error: 'too_few' | 'collinear' | 'singular' | 'duplicate'; need?: number };
const georef = op<(points: Gcp[], method: string, probes: { x: number; y: number }[], back: { x: number; y: number }[]) => Answer>('rasterGeoref');

function gcpOf(row: Row): Gcp | null {
  const n = (k: string) => {
    const v = readNumber(row[k] ?? '');
    return v === null || Number.isNaN(v) ? null : v;
  };
  const [c, r, y, x] = [n('col'), n('row'), n('ty'), n('tx')];
  if (c === null || r === null || y === null || x === null) return null;
  return { pixel: { x: c, y: r }, target: { x: y, y: x }, used: row.use !== '0' };
}

/** The raster's pixel under a point of the drawing (fractional), none when its affine does not invert. */
function pixelOf(r: RasterEntity, p: { x: number; y: number }): { x: number; y: number } | null {
  const [x0, a, b, y0, c, d] = r.affine;
  const det = a * d - b * c;
  if (!(Number.isFinite(det) && det !== 0)) return null;
  const [dx, dy] = [p.x - x0, p.y - y0];
  return { x: (d * dx - b * dy) / det, y: (a * dy - c * dx) / det };
}

/** A raster's scene key and an embedded one's bytes. */
function sourceOf(ctx: AppContext, r: RasterEntity): { key: string; url: string | null } {
  if (r.asset) return { key: rasterKey(r), url: (ctx.doc.styles.value.items.find((it) => it.id === r.asset) as { data?: string } | undefined)?.data ?? null };
  return { key: rasterKey(r), url: null };
}

class RasterGeorefDialog {
  private readonly ctx: AppContext;
  private readonly rasterBox = h('div', { class: 'io-row' });
  private readonly methodBox = h('div', { class: 'io-row' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly optionsBox = h('div', { class: 'io-row' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly apply = h('button', { class: 'btn btn--primary', type: 'button' }, 'Uygula');
  private readonly grid: Grid;
  private readonly dialog: Dialog;
  private answer: Answer | null = null;
  private rowsOfPoints: number[] = [];

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const rasters = this.rasters();
    const chosen = [...ctx.selection.ids.value].find((id) => rasters.some((r) => r.id === id));
    if (chosen !== undefined) state.raster = chosen;
    else if (state.raster === null || !rasters.some((r) => r.id === state.raster)) state.raster = rasters[0]?.id ?? null;
    const model: GridModel = {
      columns: COLUMNS,
      rows: () => state.rows,
      addLabel: 'Nokta ekle',
      readonly: (_r, key) => RESIDUALS.has(key),
      mark: (r) => this.mark(r),
      actions: (r) => [
        { icon: 'target', label: `${r + 1}. noktanın pikselini rasterde göster`, tip: 'Rasterin üstünde, noktanın göründüğü yere tıklayın.', run: () => this.pick(r, 'source') },
        { icon: 'pin', label: `${r + 1}. noktanın hedefini çizimde göster`, tip: 'Noktanın doğru yerine tıklayın; kenet çalışır.', run: () => this.pick(r, 'target') },
      ],
      canInsertAfter: () => true,
      insertAfter: (r) => state.rows.splice(r + 1, 0, {}),
      canRemove: () => state.rows.length > 1,
      remove: (r) => state.rows.splice(r, 1),
    };
    this.grid = new Grid(model, () => this.solve());
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    this.dialog = new Dialog({
      title: 'Raster oturt',
      width: 900,
      className: 'dialog--io dialog--calc dialog--fit',
      content: [this.rasterBox, this.methodBox, h('div', { class: 'calc-section' }, h('h3', { class: 'calc-results__title' }, 'Kontrol noktaları'), this.grid.el), this.summaryBox, this.optionsBox],
      footer: [this.status, close, this.apply],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.apply.addEventListener('click', () => void this.write());
    this.renderControls();
    this.solve();
  }

  private rasters(): (RasterEntity & { uid: string })[] {
    return [...this.ctx.doc.all()].filter((e): e is RasterEntity & { uid: string } => e.kind === 'raster');
  }

  private raster(): RasterEntity | null {
    const e = state.raster === null ? undefined : this.ctx.doc.get(state.raster);
    return e?.kind === 'raster' ? e : null;
  }

  private renderControls(): void {
    const layers = this.ctx.doc.layers;
    const rasters = this.rasters();
    const list = select(
      'Raster',
      rasters.length ? rasters.map((r) => ({ value: String(r.id), label: layers.path(r.layerId) })) : [{ value: '', label: 'Çizimde raster yok' }],
      state.raster === null ? '' : String(state.raster),
      (v) => {
        state.raster = v ? Number(v) : null;
        this.solve();
      },
      'raster',
    );
    const pickRaster = h('button', { class: 'ibtn', type: 'button', title: 'Rasteri çizimden seç: rasterin üstüne tıklayın', 'aria-label': 'Rasteri çizimden seç' }, icon('target', 14));
    pickRaster.addEventListener('click', () => this.pickRaster());
    replaceChildren(this.rasterBox, field('Raster', h('div', { class: 'raster-pick' }, list, pickRaster), null, 'grow'));
    const method = segmented<Method>({ label: 'Dönüşüm', options: METHODS, value: state.method, onChange: (v) => ((state.method = v), this.renderControls(), this.solve()) });
    replaceChildren(this.methodBox, field('Dönüşüm', method, HINT[state.method], 'grow'));
    this.renderOptions();
  }

  private renderOptions(): void {
    const r = this.raster();
    if (affineMethod(state.method)) {
      const linked = !!r?.file;
      replaceChildren(
        this.optionsBox,
        checkField('Dünya dosyası', 'Uygulayınca dünya dosyasını da indir', state.world, (on) => (state.world = on), 'world', linked ? 'Rasterin yanına koyun: başka yazılımlar da yerini bilir.' : 'Gömülü raster için de indirilebilir.'),
      );
      return;
    }
    const pixel = textField({ label: 'Çıktının piksel boyu (m)', value: state.pixel, placeholder: 'dönüşümden', onChange: (v) => (state.pixel = v) });
    replaceChildren(
      this.optionsBox,
      field('Çıktının piksel boyu (m)', pixel, 'Boş bırakılırsa dönüşümün noktalardaki ortalama ölçeğinden.'),
      checkField('Örnekleme', 'En yakın piksel (sınıflı rasterler için)', state.nearest, (on) => (state.nearest = on), 'nearest'),
    );
  }

  private mark(r: number): string | null {
    if (state.rows[r]?.use === '0') return 'off';
    const a = this.answer;
    if (!a || 'error' in a || a.m0 === null) return null;
    let worst = -1;
    this.rowsOfPoints.forEach((row, i) => {
      if (state.rows[row].use === '0') return;
      if (worst < 0 || a.residuals[i][2] > a.residuals[worst][2]) worst = i;
    });
    return worst >= 0 && this.rowsOfPoints[worst] === r ? 'worst' : null;
  }

  private solve(): void {
    const points: Gcp[] = [];
    this.rowsOfPoints = [];
    state.rows.forEach((row, r) => {
      row.vy = row.vx = row.v = '';
      const g = gcpOf(row);
      if (g) {
        points.push(g);
        this.rowsOfPoints.push(r);
      }
    });
    this.answer = points.length ? georef(points, state.method, [], []) : null;
    const a = this.answer;
    if (a && !('error' in a) && state.method !== 'thinPlate')
      a.residuals.forEach(([vx, vy, v], i) => {
        const row = state.rows[this.rowsOfPoints[i]];
        const t = (n: number) => (Number.isFinite(n) ? fixed(n, 3) : '—');
        [row.vy, row.vx, row.v] = [t(vx), t(vy), t(v)];
      });
    this.grid.refresh();
    const lines: HTMLElement[] = [];
    const r = this.raster();
    if (!r) lines.push(summaryLine('warn', 'Çizimde raster yok ya da seçili değil: önce bir raster seçin.'));
    else if (!a) lines.push(summaryLine('info', `Kontrol noktalarını girin: Sütun ve Satır rasterin üstünde, Hedef çizimde gösterilir. ${NAME[state.method]} en az ${NEED[state.method]} nokta ister.`));
    else if ('error' in a)
      lines.push(
        summaryLine(
          'warn',
          a.error === 'too_few'
            ? `${NAME[state.method]} en az ${a.need ?? NEED[state.method]} kullanılan nokta ister.`
            : a.error === 'collinear'
              ? 'Noktaların pikselleri bir doğru üzerinde; rasterin iki yönüne yayılmış noktalar seçin.'
              : a.error === 'duplicate'
                ? 'İki kullanılan nokta aynı pikselde; birini düzeltin ya da Kullan’dan çıkarın.'
                : 'Bu noktalarla dönüşüm çözülemiyor; noktaları rasterin geneline yayın.',
        ),
      );
    else {
      const m0 = a.m0 !== null ? `m0 = ±${fixed(a.m0, 3)} m.` : state.method === 'thinPlate' ? 'İnce plaka noktalardan tam geçer; artık yok.' : 'Fazla nokta yok: m0 hesaplanmaz.';
      lines.push(summaryLine('ok', `${NAME[state.method]} çözüldü. ${m0}`));
      lines.push(
        summaryLine(
          'info',
          affineMethod(state.method)
            ? 'Uygula rasterin dönüşümünü değiştirir (tek adım).'
            : 'Uygula rasteri yeniden örnekleyip “-oturtulmus.tif” olarak indirir; 32 MB’a kadar olanı projenin kitaplığına gömer, raster onu gösterir (tek adım).',
        ),
      );
    }
    summary(this.summaryBox, lines);
    this.apply.disabled = !r || !a || 'error' in a;
  }

  private pick(r: number, side: 'source' | 'target'): void {
    const { ctx } = this;
    const label = `${r + 1}. noktanın ${side === 'source' ? 'rasterdeki yeri' : 'hedefi'}`;
    this.dialog.close();
    ctx.tools.run(
      new PickPointTool(ctx, `Raster oturt: ${label}`, (p) => {
        const row = state.rows[r];
        const raster = this.raster();
        if (p && row) {
          if (side === 'source') {
            const q = raster ? pixelOf(raster, p) : null;
            if (q) [row.col, row.row] = [fixed(q.x, 2), fixed(q.y, 2)];
          } else [row.ty, row.tx] = [fixed(p.x, 3), fixed(p.y, 3)];
          if (!row.use) row.use = '1';
        }
        queueMicrotask(() => openRasterGeoref(ctx));
      }),
      `Raster oturt: ${label}`,
    );
  }

  private pickRaster(): void {
    const { ctx } = this;
    this.dialog.close();
    ctx.tools.run(
      new PickPointTool(ctx, 'Raster oturt: rasterin üstüne tıklayın', (p) => {
        if (p) {
          const under = this.rasters()
            .reverse()
            .find((r) => {
              const q = pixelOf(r, p);
              return q !== null && q.x >= 0 && q.y >= 0 && q.x < r.width && q.y < r.height;
            });
          if (under) state.raster = under.id;
        }
        queueMicrotask(() => openRasterGeoref(ctx));
      }),
      'Raster oturt: rasterin üstüne tıklayın',
    );
  }

  /** The raster's new fields through `cad.entities.edit`'s `rasterGeoref`, one step. */
  private writeRaster(f: RasterEntity): boolean {
    const geometry = {
      kind: 'raster',
      affine: f.affine,
      width: f.width,
      height: f.height,
      bands: f.bands,
      sample: f.sample,
      ...(f.asset ? { asset: f.asset } : { file: f.file }),
      srid: f.srid,
      style: cleanRasterStyle(f.style),
      ...(f.opacity !== undefined && { opacity: f.opacity }),
    } as unknown as EditGeometry;
    const out = writeEdit(this.ctx, 'rasterGeoref' as EditOperation, [{ kind: 'update', uid: uidOf(this.ctx, f.id), geometry }]);
    if (!out) return false;
    rasterService().forgetTiles();
    return true;
  }

  private async write(): Promise<void> {
    const { ctx } = this;
    const r = this.raster();
    const a = this.answer;
    if (!r || !a || 'error' in a) return;
    const srid = ctx.doc.settings.crs.value.srid;
    const points = state.rows.map(gcpOf).filter((g): g is Gcp => g !== null);
    if (a.affine) {
      const affine = a.affine as RasterEntity['affine'];
      if (!this.writeRaster({ ...r, affine, srid })) return;
      if (state.world) {
        const ext = (r.file ?? 'raster.tif').split('.').pop() ?? 'tif';
        const name = `${(r.file ?? r.asset ?? 'raster').replace(/\.[^.]*$/, '')}.${ext.length >= 3 ? `${ext[0]}${ext[ext.length - 1]}w` : 'wld'}`;
        download(new Blob([worldFile(affine)], { type: 'text/plain' }), name);
      }
      ctx.log.success('Raster oturt: rasterin dönüşümü değişti. Ctrl+Z geri alır.');
      this.dialog.close();
      return;
    }
    const { key, url } = sourceOf(ctx, r);
    // The file's own name, without its folder and extension; an embedded raster's layer's.
    const name = r.file ? (r.file.split(/[\\/]/).pop() ?? r.file).replace(/\.[^.]*$/, '') : (ctx.doc.layers.get(r.layerId)?.name ?? 'raster');
    const pixel = readNumber(state.pixel);
    const job = rasterService().warp(key, url, `${name}`, {
      points: points.flatMap((g) => [g.pixel.x, g.pixel.y, g.target.x, g.target.y, g.used ? 1 : 0]),
      method: state.method,
      pixel: pixel !== null && Number.isFinite(pixel) && pixel > 0 ? pixel : 0,
      nearest: state.nearest,
      epsg: srid,
    });
    this.dialog.close();
    ctx.log.info(`Raster oturt: “${name}” yeniden örnekleniyor; sağ alttaki panel ilerlemeyi gösterir.`);
    let out: { bytes: Uint8Array; grid: number[] };
    try {
      out = await job.done;
    } catch (e) {
      const why = e instanceof Error ? e.message : String(e);
      if (why === 'durduruldu') ctx.log.info('Raster oturt: durduruldu; raster değişmedi.');
      else ctx.log.warn(`Raster oturt: ${why}`);
      return;
    }
    const [x0, s, , y0, , t, width, height, bands, alpha] = out.grid;
    const fileName = `${name}-oturtulmus.tif`;
    const bytes = out.bytes;
    download(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: 'image/tiff' }), fileName);
    const next: RasterEntity = {
      ...r,
      affine: [x0, s, 0, y0, 0, t],
      width,
      height,
      bands,
      sample: alpha ? 'u8' : r.sample,
      srid,
      style: alpha ? { render: 'rgb', bands: [1, 2, 3, 4], ...(r.style.resampling && { resampling: r.style.resampling }) } : r.style,
    };
    delete next.asset;
    delete next.file;
    if (bytes.length <= MOST_EMBEDDED) {
      const id = `raster-${(await sha256Hex(bytes)).slice(0, 16)}`;
      const doc = ctx.doc;
      if (!hasRaster(doc, id))
        doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, { kind: 'asset', id, name: `${name}-oturtulmus`, path: ['Rasterler'], format: 'tiff', data: `data:image/tiff;base64,${toBase64(bytes)}`, width, height } as never] });
      next.asset = id;
    } else {
      next.file = fileName;
      rasterService().setFile(fileName, new File([bytes as Uint8Array<ArrayBuffer>], fileName, { type: 'image/tiff' }));
    }
    if (this.writeRaster(next)) ctx.log.success(`Raster oturt: “${name}” yeniden örneklendi${next.asset ? ' ve gömüldü' : `; “${fileName}” indirildi, çizim onu bağlı tutar`}. Ctrl+Z geri alır.`);
  }
}

/** A world file's text for an affine to pixel corners: A, D, B, E, then C and F at the first pixel's centre. */
function worldFile([x0, a, b, y0, c, d]: readonly number[]): string {
  const cx = x0 + a / 2 + b / 2;
  const cy = y0 + c / 2 + d / 2;
  return [a, c, b, d, cx, cy].map((v) => String(v)).join('\n') + '\n';
}
