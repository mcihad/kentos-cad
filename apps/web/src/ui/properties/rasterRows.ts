import type { AppContext } from '../../app/context';
import { fixed } from '../../core/displayNumber';
import { crsBySrid, crsTitle } from '../../geo/crs';
import type { RasterEntity, RasterRender, RasterStretch } from '../../model/entities';
import { hasRaster } from '../../product/entitiesEdit';
import { sha256Hex, toBase64 } from '../../product/sheet/store';
import { rasterService } from '../../render/rasterService';
import { MOST_EMBEDDED, RASTER_FILES, sampleLabel } from '../raster/RasterAddDialog';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometry } from './write';

/** How a look is named (Raster stili's list; the desktop's `rows::raster::render_name`). */
export const RENDER_NAME: Record<RasterRender, string> = {
  rgb: 'Renkli (RGB)',
  gray: 'Gri',
  palette: 'Paletli',
  ramp: 'Renk rampası',
  hillshade: 'Gölgeli kabartma',
  rampShade: 'Rampa ve gölge',
};
const STRETCH_NAME: Record<RasterStretch, string> = { none: 'Yok', minMax: 'En küçükten en büyüğe', percent: '%2 – %98', manual: 'Elle' };

/**
 * A raster's rows in Öznitelikler (docs/adr/0204 §8), the desktop's `properties/rows/raster.rs` the same: its file
 * (embedded, or linked and whether this session has it; Göm and Kaynağı yeniden seç), its size, bands and samples, its
 * pixel's size, its file's system, its look (Raster stili…), nodata and transparency. They write through
 * `cad.entities.edit`'s properties.
 */
export function rasterRows(ctx: AppContext, e: RasterEntity, locked: boolean): PropRow[] {
  const f = ctx.format;
  const [, a, b, , c, d] = e.affine;
  const linked = e.asset === undefined && e.file !== undefined;
  const source = sourceWords(ctx, e);
  const system = e.srid === 0 ? 'Projeninki (dosya söylemiyordu)' : (() => {
    const s = crsBySrid(e.srid);
    return s ? crsTitle(s) : `EPSG:${e.srid}`;
  })();
  const rows: PropRow[] = [
    {
      label: 'Kaynak',
      value: source,
      editor:
        linked && !locked
          ? {
              type: 'select',
              display: () => ({ text: source }),
              items: () => [
                { label: 'Kaynağı yeniden seç…', run: () => void reselect(ctx, e) },
                { label: 'Göm', run: () => void embed(ctx, e) },
              ],
            }
          : undefined,
    },
    { label: 'Boyut', value: `${e.width} × ${e.height}`, numeric: true, unit: 'piksel' },
    { label: 'Bantlar', value: `${e.bands} bant, ${sampleLabel(e.sample)}` },
    { label: 'Piksel boyu', value: `${f.length(Math.hypot(a, c), false)} × ${f.length(Math.hypot(b, d), false)}`, numeric: true, unit: f.lengthUnitLabel },
    { label: 'Sistem', value: system },
    {
      label: 'Görünüş',
      value: RENDER_NAME[e.style.render],
      editor: locked ? undefined : { type: 'select', display: () => ({ text: RENDER_NAME[e.style.render] }), items: () => [{ label: 'Raster stili…', run: () => void ctx.commands.execute('raster.style') }] },
    },
  ];
  if ((e.style.stretch ?? 'none') !== 'none') rows.push({ label: 'Gerdirme', value: STRETCH_NAME[e.style.stretch ?? 'none'] });
  rows.push({ label: 'Nodata', value: e.style.nodata !== undefined ? String(e.style.nodata) : 'Dosyanınki' });
  rows.push({
    label: 'Saydamlık',
    value: fixed((1 - (e.opacity ?? 1)) * 100, 0),
    numeric: true,
    unit: '%',
    editor: locked
      ? undefined
      : {
          type: 'number',
          commit: (t: string) => {
            const x = parseFloat(t.replace(',', '.'));
            if (Number.isFinite(x) && x >= 0 && x <= 90) setGeometry(ctx, e, { opacity: x > 0 ? 1 - x / 100 : undefined });
          },
        },
  });
  return rows;
}

/** Kaynak: an embedded raster's name and size, a linked one's file and whether this session has it. */
function sourceWords(ctx: AppContext, e: RasterEntity): string {
  if (e.asset !== undefined) {
    const it = ctx.doc.styles.value.items.find((i) => i.kind === 'asset' && i.id === e.asset);
    return it?.kind === 'asset' ? `Gömülü: ${it.name} (${it.width}×${it.height} piksel)` : 'Gömülü (kitaplıkta yok)';
  }
  if (e.file !== undefined) {
    const name = e.file.split(/[\\/]/).pop() || e.file;
    return rasterService().hasFile(e.file) ? `Bağlı: ${name}` : `Bağlı: ${name} (bu oturumda seçilmedi)`;
  }
  return '—';
}

/** Kaynağı yeniden seç: the session's file for a linked raster (the browser never keeps a path). */
async function reselect(ctx: AppContext, e: RasterEntity): Promise<void> {
  const files = await ctx.files.pickFilesForImport(RASTER_FILES);
  const file = files?.find((f) => /\.(tiff?|png|jpe?g)$/i.test(f.name));
  if (!file || e.file === undefined) return;
  rasterService().setFile(e.file, file);
  ctx.log.info(`“${file.name}” bu oturum boyunca “${e.file}” rasterinin dosyası.`);
}

/** Göm (docs/adr/0204 §2): the linked raster's file asked for, kept in the project's library, the raster made embedded, one step “Değiştir”. */
async function embed(ctx: AppContext, e: RasterEntity): Promise<void> {
  let file: File | undefined;
  if (e.file !== undefined) {
    const files = await ctx.files.pickFilesForImport(RASTER_FILES);
    file = files?.find((f) => /\.(tiff?|png|jpe?g)$/i.test(f.name));
  }
  if (!file) return;
  if (file.size > MOST_EMBEDDED) return void ctx.log.warn(`“${file.name}” ${Math.floor(file.size / (1 << 20))} MB; gömülü raster en çok ${MOST_EMBEDDED >> 20} MB olabilir. Rasteri bağlı bırakın.`);
  const bytes = new Uint8Array(await file.arrayBuffer());
  const tiff = (bytes[0] === 0x49 && bytes[1] === 0x49) || (bytes[0] === 0x4d && bytes[1] === 0x4d);
  const format = tiff ? 'tiff' : bytes[0] === 0x89 ? 'png' : 'jpeg';
  const id = `raster-${(await sha256Hex(bytes)).slice(0, 16)}`;
  const doc = ctx.doc;
  if (!hasRaster(doc, id))
    doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, { kind: 'asset', id, name: file.name.replace(/\.[^.]*$/, ''), path: ['Rasterler'], format, data: `data:image/${format};base64,${toBase64(bytes)}`, width: e.width, height: e.height } as never] });
  const now = doc.get(e.id);
  if (now?.kind === 'raster') setGeometry(ctx, now, { asset: id, file: undefined });
}
