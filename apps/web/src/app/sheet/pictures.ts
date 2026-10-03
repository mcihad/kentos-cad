import type { AssetMeta } from '../../contracts/generated/sheet/AssetMeta';
import { fixed } from '../../core/displayNumber';
import { sha256Hex } from '../../product/sheet/store';
import type { AppContext } from '../context';
import type { SheetService } from './service';

/**
 * A picture for a sheet (docs/sheet/design.md §3.4): a PNG, JPEG or SVG file
 * no larger than the engine allows, measured, named by the SHA-256 of its
 * bytes and kept on this device; the book takes its metadata only (the
 * inspector's “Resim seç…” adds it with `AddAssets` and gives the frame its
 * digest, one step).
 */
export async function addPicture(ctx: AppContext, s: SheetService, file: File): Promise<AssetMeta | null> {
  const engine = await s.ensureEngine();
  const kind = file.type === 'image/png' ? 'png' : file.type === 'image/jpeg' ? 'jpeg' : file.type === 'image/svg+xml' || /\.svg$/i.test(file.name) ? 'svg' : null;
  if (!kind) {
    ctx.log.warn(`“${file.name}” resim değil: PNG, JPEG ya da SVG seçin.`);
    return null;
  }
  if (file.size > engine.info.assetMaxBytes) {
    ctx.log.warn(`“${file.name}” ${fixed(file.size / 1048576, 1)} MB: bir resim en çok ${fixed(engine.info.assetMaxBytes / 1048576, 0)} MB olabilir. Küçültüp yeniden seçin.`);
    return null;
  }
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const size = await pictureSize(new Blob([bytes], { type: file.type || 'image/svg+xml' }), kind === 'svg');
    const meta: AssetMeta = { sha256: await sha256Hex(bytes), kind, name: file.name, width: size.width, height: size.height, bytes: bytes.length };
    await s.assets.put(meta, bytes);
    void s.paint.refreshAssets();
    return meta;
  } catch (e) {
    ctx.log.error(`“${file.name}” okunamadı: ${(e as Error).message}. Başka bir dosya deneyin.`);
    return null;
  }
}

/** A picture's size in pixels (an SVG's own width and height, as an image element reads them). */
async function pictureSize(blob: Blob, svg: boolean): Promise<{ width: number; height: number }> {
  if (!svg) {
    const bmp = await createImageBitmap(blob);
    const size = { width: bmp.width, height: bmp.height };
    bmp.close();
    return size;
  }
  const url = URL.createObjectURL(blob);
  try {
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('SVG çözülemedi'));
      img.src = url;
    });
    return { width: img.naturalWidth || 100, height: img.naturalHeight || 100 };
  } finally {
    URL.revokeObjectURL(url);
  }
}
