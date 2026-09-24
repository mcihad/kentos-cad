import type { AppContext } from '../../app/context';
import type { LibraryAsset } from '../../model/style';
import { exportStyles } from '../../style/file';
import { newItemId } from '../../style/library';

/**
 * Style files on disk (.kstil, docs/STYLE.md §5): saving a selection of the
 * library with the drawings it uses, and reading one back.
 */

const fileSlug = (s: string) =>
  s
    .toLocaleLowerCase('tr')
    .replace(/[çğıöşü]/g, (c) => ({ ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u' })[c] ?? c)
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'stiller';

/** Downloads the items (and the assets their symbols draw with) as `<name>.kstil`. */
export function downloadStyles(ctx: AppContext, ids: readonly string[], name: string): number {
  const file = exportStyles(ctx.styles.library, ids);
  const blob = new Blob([JSON.stringify(file, null, 1)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${fileSlug(name)}.kstil`;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return file.items.length;
}

/**
 * Asks for a .kstil file (read as text) or a PNG/JPEG picture (made into a
 * raster asset for image fills and markers); null when cancelled.
 */
export function pickStyleFile(): Promise<{ name: string; text: string } | { image: LibraryAsset } | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.kstil,.json,application/json,image/png,image/jpeg';
    input.addEventListener('change', () => {
      const f = input.files?.[0];
      if (!f) return resolve(null);
      if (/^image\/(png|jpeg)$/.test(f.type)) return void rasterAsset(f).then(resolve, () => resolve(null));
      f.text().then(
        (text) => resolve({ name: f.name, text }),
        () => resolve(null),
      );
    });
    input.addEventListener('cancel', () => resolve(null));
    input.click();
  });
}

/** A PNG or JPEG file as a raster asset (data URL and natural size), for raster patterns. */
export async function rasterAsset(f: File): Promise<{ image: LibraryAsset }> {
  const data = await new Promise<string>((ok, fail) => {
    const r = new FileReader();
    r.onload = () => ok(String(r.result));
    r.onerror = () => fail(r.error);
    r.readAsDataURL(f);
  });
  const img = new Image();
  img.src = data;
  await img.decode();
  return {
    image: { kind: 'asset', id: newItemId('a'), name: f.name.replace(/\.(png|jpe?g)$/i, ''), path: ['Görüntülerim'], format: f.type === 'image/png' ? 'png' : 'jpeg', data, width: img.naturalWidth, height: img.naturalHeight },
  };
}
