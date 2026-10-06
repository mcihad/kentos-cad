import type { LibraryAsset } from '../model/style';
import { sha256Hex, toBase64 } from '../product/sheet/store';

/**
 * Resim ekle's file (docs/adr/0192 §2), twin of the desktop's `pictures::library_item`: a PNG or JPEG as an image of
 * the project's library, its id from its content (`resim-` and its SHA-256's first sixteen hex digits, so the same
 * picture is kept once), its name the file's stem, under Resimler; why not when it is neither, too large or its
 * header cannot be read. The size is the header's; a body that does not decode draws grey (render/pictures.ts).
 */

/** The largest file a picture is read from. */
export const MOST_BYTES = 32 * 1024 * 1024;

/** The picture files Resim ekle offers to open. */
export const PICTURE_FILES = { description: 'Resim (PNG, JPEG)', accept: { 'image/png': ['.png'], 'image/jpeg': ['.jpg', '.jpeg'] } };

const be16 = (b: Uint8Array, i: number) => (b[i] << 8) | b[i + 1];
const be32 = (b: Uint8Array, i: number) => ((b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3]) >>> 0;

/** A PNG's width and height from its IHDR; null when it is not there. */
export function pngSize(b: Uint8Array): [number, number] | null {
  if (b.length < 24 || String.fromCharCode(...b.subarray(12, 16)) !== 'IHDR') return null;
  const w = be32(b, 16);
  const h = be32(b, 20);
  return w > 0 && h > 0 ? [w, h] : null;
}

/** A JPEG's width and height from its first frame header (SOF0–SOF15 but DHT, JPG and DAC); null when there is none. */
export function jpegSize(b: Uint8Array): [number, number] | null {
  let i = 2;
  while (i + 3 < b.length) {
    if (b[i] !== 0xff) return null;
    while (b[i] === 0xff && i < b.length) i++;
    const marker = b[i];
    // Markers without a length.
    if (marker === 0x01 || (marker >= 0xd0 && marker <= 0xd8)) {
      i++;
      continue;
    }
    if (marker === 0xd9 || i + 2 >= b.length) return null;
    const length = be16(b, i + 1);
    if (marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc) {
      if (i + 8 >= b.length) return null;
      const h = be16(b, i + 4);
      const w = be16(b, i + 6);
      return w > 0 && h > 0 ? [w, h] : null;
    }
    i += 1 + length;
  }
  return null;
}

/** A file's stem: its name without a .png, .jpg or .jpeg ending. */
const stem = (name: string) => name.replace(/\.(png|jpe?g)$/i, '');

export type PictureItem = { ok: true; id: string; item: LibraryAsset } | { ok: false; error: string };

export async function pictureItem(name: string, bytes: Uint8Array): Promise<PictureItem> {
  if (bytes.length > MOST_BYTES) return { ok: false, error: `“${name}” ${Math.floor(bytes.length / (1024 * 1024))} MB; resim en çok ${MOST_BYTES / (1024 * 1024)} MB olabilir.` };
  const png = bytes.length >= 4 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47;
  const jpeg = bytes.length >= 2 && bytes[0] === 0xff && bytes[1] === 0xd8;
  if (!png && !jpeg) return { ok: false, error: `“${name}” PNG ya da JPEG değil; resim olarak eklenemez.` };
  const format = png ? 'png' : 'jpeg';
  const size = png ? pngSize(bytes) : jpegSize(bytes);
  if (!size) return { ok: false, error: `“${name}” okunamadı: ${format.toUpperCase()} dosyası bozuk görünüyor.` };
  const id = `resim-${(await sha256Hex(bytes)).slice(0, 16)}`;
  const item: LibraryAsset = { kind: 'asset', id, name: stem(name), path: ['Resimler'], format, data: `data:image/${format};base64,${toBase64(bytes)}`, width: size[0], height: size[1] };
  return { ok: true, id, item };
}
