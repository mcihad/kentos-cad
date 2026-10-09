import { analyzePoints, analyzeRaster } from '../io/rasterAnalysis';
import type { RasterEntity } from '../model/entities';
import { hasRaster } from '../product/entitiesEdit';
import { setRasterRunHost } from '../processing/rasterHost';
import { rasterService } from '../render/rasterService';
import type { AppContext } from './context';

/** An embedded raster's most bytes (ADR 0204 §8, Raster ekle's `MOST_EMBEDDED`). */
const MOST_EMBEDDED = 32 * 1024 * 1024;

/** The browser saves `blob` as `name` (the downloads folder). */
function download(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * The page's host for the raster tools (Yüzey analizi, docs/adr/0231 §2; İnterpolasyon and Yoğunluk, docs/adr/0232;
 * processing/rasterHost.ts): a raster's bytes
 * from the raster service (the session's linked file, an embedded raster's library data), the job in a worker of its
 * own (io/rasterAnalysis.ts), and the result kept as ADR 0204 §8 keeps a raster: embedded in the project's library up
 * to 32 MB (an edit of the library, not an undo step), else the session's file of its name, downloaded.
 */
export function installRasterAnalysis(ctx: AppContext): void {
  setRasterRunHost({
    source(r: RasterEntity) {
      if (r.url && !r.asset && !r.file) return { refused: 'Adresteki raster web’de çözümlenmiyor; dosyasını indirip Raster ekle ile ekleyin.' };
      const key = r.asset ? `asset:${r.asset}` : `file:${r.file ?? ''}`;
      const url = r.asset ? ((ctx.doc.styles.value.items.find((it) => it.id === r.asset) as { data?: string } | undefined)?.data ?? null) : null;
      const src = rasterService().source(key, url);
      if (src) return src.blob;
      return { refused: r.asset ? `“${r.asset}” kimlikli raster projenin kitaplığında yok.` : `“${r.file}” bu oturumda verilmedi: rasteri seçip Öznitelikler'de Kaynağı yeniden seç ile dosyasını verin.` };
    },
    analyze: analyzeRaster,
    analyzePoints,
    async keep(bytes, name, width, height) {
      if (bytes.length <= MOST_EMBEDDED) {
        const { sha256Hex, toBase64 } = await import('../product/sheet/store');
        const id = `raster-${(await sha256Hex(bytes)).slice(0, 16)}`;
        const doc = ctx.doc;
        if (!hasRaster(doc, id))
          doc.styles.set({
            ...doc.styles.value,
            items: [...doc.styles.value.items, { kind: 'asset', id, name: name.replace(/\.tif$/i, ''), path: ['Rasterler'], format: 'tiff', data: `data:image/tiff;base64,${toBase64(bytes)}`, width, height } as never],
          });
        return { asset: id, note: `“${name}” projeye gömüldü.` };
      }
      rasterService().setFile(name, new File([bytes as Uint8Array<ArrayBuffer>], name, { type: 'image/tiff' }));
      download(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: 'image/tiff' }), name);
      return { file: name, note: `“${name}” 32 MB'tan büyük: indirildi; çizim onu bu oturumda bağlı tutar.` };
    },
  });
}
